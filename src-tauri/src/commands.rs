//! Tauri command boundary: validation, locking and background work.

use gambiit_agent::{AgentSession, OpenCodeClient};
use parking_lot::Mutex;
use shakmaty::Color;
use std::sync::Arc;
use tauri::State;

use crate::session::{ClockUpdate, DrawOfferOutcome, GameSnapshot, Session, parse_color};
use crate::settings::{
    Settings, credential_env_for_model, director_agent_config, ui_language_name,
};
use crate::{analysis, credentials, director, leaderboard, mcp, pgn, pieces, storage};

pub(crate) struct AppState {
    session: Arc<Mutex<Session>>,
    mcp: Option<Arc<mcp::Bridge>>,
}

impl AppState {
    pub(crate) fn new(session: Arc<Mutex<Session>>, mcp: Option<Arc<mcp::Bridge>>) -> Self {
        Self { session, mcp }
    }
}

pub(crate) async fn run_coach(
    session: Arc<Mutex<Session>>,
    action: &str,
    level: Option<u8>,
) -> Result<GameSnapshot, String> {
    let job = { session.lock().prepare_coach(action, level)? };
    let worker = job.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        let mut agent =
            AgentSession::new(worker.config.clone()).map_err(|error| error.to_string())?;
        let (text, output) = agent
            .request_text(&worker.prompt)
            .map_err(|error| error.to_string())?;
        Ok::<_, String>((text, output.tokens, output.cost))
    })
    .await;
    match joined {
        Ok(result) => session.lock().finish_coach(&job, result),
        Err(error) => {
            session.lock().coach_thinking = false;
            Err(error.to_string())
        }
    }
}

#[tauri::command]
pub(crate) async fn coach_explain(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    run_coach(state.session.clone(), "explain", None).await
}

#[tauri::command]
pub(crate) async fn coach_hint(
    state: State<'_, AppState>,
    level: u8,
) -> Result<GameSnapshot, String> {
    run_coach(state.session.clone(), "hint", Some(level)).await
}

#[tauri::command]
pub(crate) async fn coach_review(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    run_coach(state.session.clone(), "review", None).await
}

#[tauri::command]
pub(crate) fn coach_clear(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().clear_coach()
}

#[tauri::command]
pub(crate) async fn director_cut(
    state: State<'_, AppState>,
    report: analysis::AnalysisReport,
) -> Result<director::DirectorCut, String> {
    let (context, settings, game_id, history_len) = {
        let session = state.session.lock();
        let settings = session.settings.clone();
        let context = director::Context {
            game: session.game.clone(),
            quotes: session
                .commentary
                .iter()
                .map(|card| director::Quote {
                    uci: card.uci.clone(),
                    text: card.comment.clone(),
                })
                .collect(),
            chat: session
                .chat
                .iter()
                .filter(|line| line.role != "system")
                .map(|line| director::ChatLine {
                    role: line.role.clone(),
                    text: line.text.clone(),
                })
                .collect(),
            persona: settings.persona_name(),
            model: settings.model.clone(),
            ui_language: ui_language_name(&settings.ui_language).to_string(),
        };
        (
            context,
            settings,
            session.game_id.clone(),
            session.game.history().len(),
        )
    };

    let plan = director::prepare(&context, &report)?;
    let config = director_agent_config(&settings, credential_env_for_model(&settings.model));
    let worker_plan = plan.clone();
    let worker_config = config.clone();
    let worker_model = settings.model.clone();
    let cut = tauri::async_runtime::spawn_blocking(move || {
        let mut agent = AgentSession::new(worker_config).map_err(|error| error.to_string())?;
        let mut total_tokens = 0u64;
        let mut total_cost = 0f64;
        let mut last_error = String::new();
        for attempt in 0..2 {
            let prompt = if attempt == 0 {
                worker_plan.prompt.clone()
            } else {
                director::retry_prompt(&last_error)
            };
            let (text, output) = agent
                .request_text(&prompt)
                .map_err(|error| error.to_string())?;
            total_tokens += output.tokens.unwrap_or(0);
            total_cost += output.cost.unwrap_or(0.0);
            match director::finish(
                &worker_plan,
                &text,
                &worker_model,
                (total_tokens > 0).then_some(total_tokens),
                (total_cost > 0.0).then_some(total_cost),
            ) {
                Ok(cut) => return Ok(cut),
                Err(error) => last_error = error,
            }
        }
        Err(format!(
            "The Director's Cut could not be generated: {last_error}"
        ))
    })
    .await
    .map_err(|error| error.to_string())??;

    let current = {
        let session = state.session.lock();
        (session.game_id.clone(), session.game.history().len())
    };
    if current.0 != game_id || current.1 != history_len {
        return Err(
            "The game changed while the Director's Cut was being written. Run it again."
                .to_string(),
        );
    }
    Ok(cut)
}

#[tauri::command]
pub(crate) fn get_state(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    Ok(state.session.lock().snapshot())
}

#[tauri::command]
pub(crate) fn clock_tick(state: State<'_, AppState>) -> Result<ClockUpdate, String> {
    let mut session = state.session.lock();
    session.tick_clock();
    let game_over = session.game.status().is_over();
    Ok(ClockUpdate {
        game_id: session.game_id.clone(),
        clock: session.clock_view_for(game_over),
        game_over,
    })
}

#[tauri::command]
pub(crate) fn new_game(
    state: State<'_, AppState>,
    settings: Settings,
    human_color: String,
) -> Result<GameSnapshot, String> {
    let color = parse_color(&human_color)
        .ok_or_else(|| "human_color must be white or black".to_string())?;
    let mut session = state.session.lock();
    *session = Session::new(settings, color);
    Ok(session.snapshot())
}

#[tauri::command]
pub(crate) fn update_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<GameSnapshot, String> {
    state.session.lock().update_settings(settings)
}

#[tauri::command]
pub(crate) fn make_move(state: State<'_, AppState>, mv: String) -> Result<GameSnapshot, String> {
    state.session.lock().make_move(&mv)
}

#[tauri::command]
pub(crate) async fn agent_move(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    let session = state.session.clone();
    let bridge = state.mcp.clone();
    tauri::async_runtime::spawn_blocking(move || session.lock().agent_move_with_mcp(bridge))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn chat_send(
    state: State<'_, AppState>,
    message: String,
) -> Result<GameSnapshot, String> {
    let session = state.session.clone();
    let bridge = state.mcp.clone();
    tauri::async_runtime::spawn_blocking(move || {
        session.lock().chat_send_with_mcp(&message, bridge)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) fn clear_chat(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().clear_chat()
}

#[tauri::command]
pub(crate) async fn offer_draw(state: State<'_, AppState>) -> Result<DrawOfferOutcome, String> {
    let session = state.session.clone();
    let bridge = state.mcp.clone();
    tauri::async_runtime::spawn_blocking(move || session.lock().offer_draw_with_mcp(bridge))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) fn takeback(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().takeback()
}

#[tauri::command]
pub(crate) fn resign(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().resign()
}

#[tauri::command]
pub(crate) fn list_models() -> Result<Vec<String>, String> {
    let client = OpenCodeClient::new(10).map_err(|error| error.to_string())?;
    let output = std::process::Command::new(client.binary())
        .arg("models")
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut models: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains(' '))
        .map(str::to_string)
        .collect();
    models.sort();
    models.dedup();
    Ok(models)
}

#[tauri::command]
pub(crate) fn list_games() -> Result<Vec<storage::GameSummary>, String> {
    storage::list()
}

#[tauri::command]
pub(crate) fn load_game(state: State<'_, AppState>, id: String) -> Result<GameSnapshot, String> {
    let saved = storage::load(&id)?;
    let mut session = Session::from_saved(saved)?;
    let snapshot = session.snapshot();
    *state.session.lock() = session;
    Ok(snapshot)
}

#[tauri::command]
pub(crate) fn delete_game(id: String) -> Result<(), String> {
    storage::delete(&id)
}

#[tauri::command]
pub(crate) fn rename_game(id: String, name: String) -> Result<storage::GameSummary, String> {
    storage::rename(&id, &name)
}

#[tauri::command]
pub(crate) fn get_pgn(state: State<'_, AppState>) -> Result<String, String> {
    let session = state.session.lock();
    Ok(pgn::export(&session.game, &session.settings))
}

#[tauri::command]
pub(crate) fn export_pgn(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let session = state.session.lock();
    let pgn = pgn::export(&session.game, &session.settings);
    let name = if name.trim().is_empty() {
        session.game_name.clone()
    } else {
        name
    };
    storage::save_pgn(&name, &pgn)
}

#[tauri::command]
pub(crate) fn import_pgn(
    state: State<'_, AppState>,
    pgn: String,
    settings: Settings,
) -> Result<GameSnapshot, String> {
    let imported = pgn::parse(&pgn)?;
    let mut session =
        Session::from_import(imported.game, settings, imported.commentary, imported.name);
    let snapshot = session.snapshot();
    *state.session.lock() = session;
    Ok(snapshot)
}

#[tauri::command]
pub(crate) async fn analyze_game(
    state: State<'_, AppState>,
    depth: u32,
) -> Result<analysis::AnalysisReport, String> {
    let (game_id, game, human_color) = {
        let session = state.session.lock();
        if session.game.history().is_empty() {
            return Err("There are no moves to analyze yet.".to_string());
        }
        (
            session.game_id.clone(),
            session.game.clone(),
            session.game.human_color(),
        )
    };
    let report = tauri::async_runtime::spawn_blocking(move || analysis::analyze(&game, depth))
        .await
        .map_err(|error| error.to_string())??;
    let summary = analysis::AnalysisSummary::from_report(&report, human_color == Color::Black);
    if let Err(error) = storage::save_analysis(&game_id, &summary) {
        eprintln!("Could not save analysis summary: {error}");
    }
    let mut session = state.session.lock();
    if session.game_id == game_id && session.game.history().len() == report.positions.len() {
        session.analysis = Some(summary);
    }
    Ok(report)
}

#[tauri::command]
pub(crate) fn get_leaderboard(
    difficulty: Option<String>,
) -> Result<leaderboard::Leaderboard, String> {
    leaderboard::build(difficulty.as_deref())
}

#[tauri::command]
pub(crate) fn list_piece_sets() -> Result<Vec<pieces::PieceSetSummary>, String> {
    pieces::list()
}

#[tauri::command]
pub(crate) fn load_piece_set(
    id: String,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    pieces::load(&id)
}

#[tauri::command]
pub(crate) fn open_pieces_folder() -> Result<(), String> {
    pieces::open_folder()
}

#[tauri::command]
pub(crate) fn credential_status(provider: String) -> credentials::CredentialStatus {
    credentials::status(&provider)
}

#[tauri::command]
pub(crate) fn save_credential(
    provider: String,
    env_var: String,
    key: String,
) -> Result<credentials::CredentialStatus, String> {
    credentials::set(&provider, &env_var, &key)
}

#[tauri::command]
pub(crate) fn delete_credential(provider: String) -> Result<credentials::CredentialStatus, String> {
    credentials::delete(&provider)
}
