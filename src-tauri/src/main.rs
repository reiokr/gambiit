#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis;
mod api;
mod bluff;
mod coach;
mod commands;
mod credentials;
mod director;
mod leaderboard;
mod mcp;
mod pgn;
mod pieces;
mod session;
mod settings;
mod storage;
mod style;

use parking_lot::Mutex;
use shakmaty::Color;
use std::sync::Arc;

pub(crate) use settings::{CustomPersona, Settings};

pub(crate) use session::{ChatMessage, CommentaryCard, GameSnapshot, SavedClock, Session};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let bridge = match mcp::start() {
        Ok(bridge) => Some(bridge),
        Err(error) => {
            eprintln!("MCP bridge unavailable: {error}");
            None
        }
    };
    let session = Arc::new(Mutex::new(Session::new_with_persist(
        Settings::default(),
        Color::White,
        false,
    )));
    let local_api = match api::start(session.clone()) {
        Ok(api) => {
            eprintln!(
                "Local read-only API: {} (discovery: {})",
                api.url,
                api.discovery_path.display()
            );
            Some(api)
        }
        Err(error) => {
            eprintln!("Local API unavailable: {error}");
            None
        }
    };
    let app = tauri::Builder::default()
        .manage(commands::AppState::new(session, bridge))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::clock_tick,
            commands::new_game,
            commands::update_settings,
            commands::make_move,
            commands::agent_move,
            commands::chat_send,
            commands::clear_chat,
            commands::coach_explain,
            commands::coach_hint,
            commands::coach_review,
            commands::coach_clear,
            commands::offer_draw,
            commands::takeback,
            commands::resign,
            commands::list_models,
            commands::list_games,
            commands::load_game,
            commands::delete_game,
            commands::rename_game,
            commands::get_pgn,
            commands::export_pgn,
            commands::import_pgn,
            commands::analyze_game,
            commands::director_cut,
            commands::get_leaderboard,
            commands::list_piece_sets,
            commands::load_piece_set,
            commands::open_pieces_folder,
            commands::credential_status,
            commands::save_credential,
            commands::delete_credential
        ])
        .build(tauri::generate_context!())
        .expect("error while building Gambiit");
    let mut local_api = local_api;
    app.run(move |_handle, event| {
        if matches!(event, tauri::RunEvent::Exit)
            && let Some(api) = local_api.take()
        {
            api.cleanup();
        }
    });
}

fn main() {
    if std::env::args().any(|argument| argument == "--mcp-server") {
        if let Err(error) = mcp::run_stdio() {
            eprintln!("Gambiit MCP server failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    run();
}

#[cfg(test)]
mod tests;
