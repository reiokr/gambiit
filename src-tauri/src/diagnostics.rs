//! On-demand setup diagnostics for the Settings panel.
//!
//! The checks are intentionally explicit and side-effect-light: they report
//! what is installed, which model is configured and whether game storage can be
//! written. The UI decides how to present required and optional checks.

use gambiit_agent::OpenCodeClient;
use gambiit_engine::Stockfish;
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{credentials, settings::Settings, storage};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetupCheck {
    pub(crate) key: &'static str,
    pub(crate) ok: bool,
    pub(crate) required: bool,
    pub(crate) path: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetupDiagnostics {
    pub(crate) app_version: String,
    pub(crate) overall_ok: bool,
    pub(crate) checks: Vec<SetupCheck>,
}

pub(crate) fn collect(settings: &Settings, mcp_available: bool) -> SetupDiagnostics {
    let checks = vec![
        check_opencode(),
        check_stockfish(),
        check_model(settings),
        check_credentials(settings),
        check_storage(),
        check_mcp(mcp_available),
    ];
    SetupDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        overall_ok: checks
            .iter()
            .filter(|check| check.required)
            .all(|check| check.ok),
        checks,
    }
}

fn check_opencode() -> SetupCheck {
    let mut check = SetupCheck {
        key: "opencode",
        ok: false,
        required: true,
        path: None,
        version: None,
        detail: None,
    };
    match OpenCodeClient::new(5) {
        Ok(client) => {
            let path = client.binary().clone();
            check.path = Some(path.display().to_string());
            check.version = command_version(&path);
            if check.version.is_none() {
                check.detail = Some("Binary found, but the version check failed.".to_string());
            }
            check.ok = true;
        }
        Err(error) => {
            check.detail = Some(error.to_string());
        }
    }
    check
}

fn check_stockfish() -> SetupCheck {
    let mut check = SetupCheck {
        key: "stockfish",
        ok: false,
        required: false,
        path: None,
        version: None,
        detail: None,
    };
    match Stockfish::discover() {
        Some(path) => {
            check.path = Some(path.display().to_string());
            match Stockfish::new(&path) {
                Ok(engine) => {
                    check.version = Some(engine.id().to_string());
                    check.ok = true;
                }
                Err(error) => {
                    check.detail = Some(error.to_string());
                }
            }
        }
        None => {
            check.detail = Some("Stockfish was not found on PATH or in STOCKFISH_BIN.".to_string());
        }
    }
    check
}

fn check_model(settings: &Settings) -> SetupCheck {
    let model = settings.model.trim();
    let ok = !model.is_empty();
    SetupCheck {
        key: "model",
        ok,
        required: true,
        path: None,
        version: None,
        detail: if ok {
            Some(format!("{model} · {} mode", settings.agent_mode))
        } else {
            Some("No model is configured.".to_string())
        },
    }
}

fn check_credentials(settings: &Settings) -> SetupCheck {
    let provider = settings
        .model
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let status = credentials::status(&provider);
    let detail = if status.has_key {
        format!(
            "API key stored locally for {} via {}.",
            status.provider,
            status
                .env_var
                .as_deref()
                .unwrap_or("its environment variable")
        )
    } else {
        "No local key; OpenCode's own authentication will be used.".to_string()
    };
    SetupCheck {
        key: "credentials",
        ok: true,
        required: false,
        path: None,
        version: None,
        detail: Some(detail),
    }
}

fn check_storage() -> SetupCheck {
    let directory = storage::games_dir();
    let mut check = SetupCheck {
        key: "storage",
        ok: false,
        required: true,
        path: Some(directory.display().to_string()),
        version: None,
        detail: None,
    };
    if let Err(error) = fs::create_dir_all(&directory) {
        check.detail = Some(format!("Could not create the game directory: {error}"));
        return check;
    }
    let probe = directory.join(".write-check");
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            check.ok = true;
        }
        Err(error) => {
            check.detail = Some(format!("Game directory is not writable: {error}"));
        }
    }
    check
}

fn check_mcp(available: bool) -> SetupCheck {
    SetupCheck {
        key: "mcp",
        ok: available,
        required: false,
        path: None,
        version: None,
        detail: Some(if available {
            "Local MCP bridge is listening.".to_string()
        } else {
            "MCP is unavailable; prompt mode still works.".to_string()
        }),
    }
}

fn command_version(path: &Path) -> Option<String> {
    let mut child = Command::new(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(50));
    };
    if !status.success() {
        return None;
    }
    let mut text = String::new();
    child.stdout.take()?.read_to_string(&mut text).ok()?;
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(120).collect())
}
