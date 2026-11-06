//! Read-only local HTTP API and SSE stream.
//!
//! The API binds only to `127.0.0.1`, requires a random bearer token and exposes
//! no mutation endpoints. It is intended for local overlays, scripts, tests and
//! a future browser shell. Discovery data is written to
//! `$XDG_RUNTIME_DIR/gambiit/session.json` with mode 0600.

use std::{
    convert::Infallible,
    env, fs,
    io::Write,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    process,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use futures_util::Stream;
use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::watch;
use tower_http::cors::{AllowOrigin, CorsLayer};
use uuid::Uuid;

use crate::Session;

const API_VERSION: u32 = 1;
const STATE_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
struct ApiState {
    session: Arc<Mutex<Session>>,
    token: Arc<String>,
    latest: watch::Sender<String>,
}

pub(crate) struct LocalApi {
    pub(crate) url: String,
    pub(crate) discovery_path: PathBuf,
}

impl LocalApi {
    pub(crate) fn cleanup(&self) {
        let _ = fs::remove_file(&self.discovery_path);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Discovery {
    url: String,
    port: u16,
    token: String,
    pid: u32,
    version: String,
    api_version: u32,
    started_at: u64,
}

/// Start the read-only API on a random localhost port. The returned handle can
/// be used to remove the discovery file when the app exits.
pub(crate) fn start(session: Arc<Mutex<Session>>) -> Result<LocalApi, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("Could not bind the local API: {error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Could not configure the local API listener: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("Could not read the local API address: {error}"))?;
    let token = new_token();
    let (initial, initial_fingerprint) = {
        let mut session = session.lock();
        let fingerprint = session.state_fingerprint();
        (
            serde_json::to_string(&session.snapshot()).unwrap_or_default(),
            fingerprint,
        )
    };
    let (latest, _) = watch::channel(initial);
    let state = ApiState {
        session,
        token: Arc::new(token.clone()),
        latest: latest.clone(),
    };
    let app = router(state.clone());
    let discovery_path = discovery_path();
    write_discovery(&discovery_path, addr, &token)?;

    let thread = std::thread::Builder::new()
        .name("gambiit-api".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("Gambiit API runtime failed: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                let listener = match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => listener,
                    Err(error) => {
                        eprintln!("Gambiit API listener failed: {error}");
                        return;
                    }
                };
                let poll_state = state.clone();
                tokio::spawn(async move {
                    let mut last_fingerprint = initial_fingerprint;
                    loop {
                        if let Some(json) = snapshot_if_changed(&poll_state, &mut last_fingerprint)
                            && !json.is_empty()
                        {
                            // `send_replace` also stores the value while no
                            // SSE client is subscribed, so a later client sees
                            // the latest state instead of the startup snapshot.
                            poll_state.latest.send_replace(json);
                        }
                        tokio::time::sleep(STATE_POLL_INTERVAL).await;
                    }
                });
                if let Err(error) = axum::serve(listener, app).await {
                    eprintln!("Gambiit API server error: {error}");
                }
            });
        })
        .map_err(|error| format!("Could not start the local API thread: {error}"))?;

    // If the thread could not be started, the discovery file should not survive.
    if thread.is_finished() {
        let _ = fs::remove_file(&discovery_path);
        return Err("The local API thread stopped immediately.".to_string());
    }

    Ok(LocalApi {
        url: format!("http://{addr}"),
        discovery_path,
    })
}

fn snapshot_if_changed(state: &ApiState, last_fingerprint: &mut u64) -> Option<String> {
    let mut session = state.session.lock();
    let fingerprint = session.state_fingerprint();
    if fingerprint == *last_fingerprint {
        return None;
    }
    *last_fingerprint = fingerprint;
    Some(serde_json::to_string(&session.snapshot()).unwrap_or_default())
}

fn router(state: ApiState) -> Router {
    let origins = vec![
        HeaderValue::from_static("tauri://localhost"),
        HeaderValue::from_static("http://tauri.localhost"),
        HeaderValue::from_static("https://tauri.localhost"),
    ];
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET])
        .allow_headers([header::AUTHORIZATION]);

    let protected = Router::new()
        .route("/api/state", get(get_state))
        .route("/api/events", get(events))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth));

    Router::new()
        .route("/api/health", get(health))
        .merge(protected)
        .layer(cors)
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "name": "gambiit",
        "version": env!("CARGO_PKG_VERSION"),
        "apiVersion": API_VERSION,
        "readOnly": true,
    }))
}

async fn get_state(State(state): State<ApiState>) -> Response {
    let body = {
        let mut session = state.session.lock();
        serde_json::to_string(&session.snapshot())
    };
    match body {
        Ok(body) => (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            )],
            body,
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn events(
    State(state): State<ApiState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = state.latest.subscribe();
    let initial = receiver.borrow_and_update().clone();
    let stream = stream! {
        if !initial.is_empty() {
            yield Ok(Event::default().event("state").data(initial));
        }
        loop {
            if receiver.changed().await.is_err() {
                break;
            }
            let json = receiver.borrow_and_update().clone();
            if !json.is_empty() {
                yield Ok(Event::default().event("state").data(json));
            }
        }
    };
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

async fn auth(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let authorized = extract_token(request.headers(), request.uri().query())
        .is_some_and(|token| constant_time_eq(token.as_bytes(), state.token.as_bytes()));
    if !authorized {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(request).await)
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn extract_token<'a>(headers: &'a HeaderMap, query: Option<&'a str>) -> Option<&'a str> {
    if let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        && let Some(token) = value.strip_prefix("Bearer ")
    {
        return Some(token);
    }
    query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("token="))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left, right) in left.iter().zip(right) {
        difference |= left ^ right;
    }
    difference == 0
}

fn new_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn discovery_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::temp_dir().join("gambiit-runtime"))
        .join("gambiit")
}

fn discovery_path() -> PathBuf {
    discovery_dir().join("session.json")
}

fn write_discovery(path: &Path, addr: SocketAddr, token: &str) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "The discovery path has no parent directory.".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("Could not create the discovery directory: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Could not secure the discovery directory: {error}"))?;
    }

    let discovery = Discovery {
        url: format!("http://{addr}"),
        port: addr.port(),
        token: token.to_string(),
        pid: process::id(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        api_version: API_VERSION,
        started_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    };
    let json = serde_json::to_string_pretty(&discovery)
        .map_err(|error| format!("Could not serialize the discovery data: {error}"))?;
    let temporary = directory.join(".session.json.tmp");
    {
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("Could not create the discovery file: {error}"))?;
        file.write_all(json.as_bytes())
            .map_err(|error| format!("Could not write the discovery file: {error}"))?;
        file.sync_all().ok();
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("Could not publish the discovery file: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use shakmaty::Color;
    use tower::ServiceExt;

    #[test]
    fn tokens_are_extracted_from_headers_and_query() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer header-token"),
        );
        assert_eq!(extract_token(&headers, None), Some("header-token"));
        assert_eq!(
            extract_token(&HeaderMap::new(), Some("a=1&token=query-token")),
            Some("query-token")
        );
        assert_eq!(extract_token(&HeaderMap::new(), None), None);
    }

    #[test]
    fn token_comparison_is_constant_time_and_exact() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreu"));
        assert!(!constant_time_eq(b"secret", b"secret-longer"));
        assert!(!constant_time_eq(b"", b"x"));
    }

    #[test]
    fn discovery_file_has_private_permissions_and_expected_shape() {
        let directory = env::temp_dir().join(format!(
            "gambiit-api-test-{}-{}",
            process::id(),
            Uuid::new_v4().simple()
        ));
        let path = directory.join("session.json");
        let addr: SocketAddr = "127.0.0.1:43210".parse().unwrap();
        write_discovery(&path, addr, "test-token").unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["port"], 43210);
        assert_eq!(value["token"], "test-token");
        assert_eq!(value["apiVersion"], 1);
        assert_eq!(value["pid"], process::id());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let _ = fs::remove_dir_all(&directory);
    }

    fn test_state() -> (ApiState, String) {
        let session = Arc::new(Mutex::new(Session::new_with_persist(
            crate::Settings::default(),
            Color::White,
            false,
        )));
        let token = "router-token".to_string();
        let (latest, _) = watch::channel(String::new());
        (
            ApiState {
                session,
                token: Arc::new(token.clone()),
                latest,
            },
            token,
        )
    }

    #[test]
    fn poll_skips_unchanged_state_and_keeps_latest_for_late_subscribers() {
        let (state, _) = test_state();
        let mut last = state.session.lock().state_fingerprint();
        assert!(snapshot_if_changed(&state, &mut last).is_none());

        state.session.lock().make_move("e2e4").unwrap();
        let json = snapshot_if_changed(&state, &mut last).expect("changed state");
        assert!(json.contains("\"turn\":\"black\""));
        assert!(snapshot_if_changed(&state, &mut last).is_none());

        state.latest.send_replace(json.clone());
        assert_eq!(*state.latest.subscribe().borrow(), json);
    }

    async fn status(app: Router, request: HttpRequest<Body>) -> StatusCode {
        app.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn state_response_is_json() {
        let (state, token) = test_state();
        let app = router(state);
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/state")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["turn"], "white");
    }

    #[tokio::test]
    async fn health_is_public_and_state_requires_a_token() {
        let (state, token) = test_state();
        let app = router(state);

        let health = status(
            app.clone(),
            HttpRequest::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(health, StatusCode::OK);

        let unauthorized = status(
            app.clone(),
            HttpRequest::builder()
                .uri("/api/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(unauthorized, StatusCode::UNAUTHORIZED);

        let authorized = status(
            app.clone(),
            HttpRequest::builder()
                .uri("/api/state")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(authorized, StatusCode::OK);

        let query_token = status(
            app,
            HttpRequest::builder()
                .uri(format!("/api/state?token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(query_token, StatusCode::OK);
    }
}
