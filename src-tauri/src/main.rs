#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use gambiit_agent::{AgentConfig, AgentSession, AgentTurn, OpenCodeClient};
use gambiit_core::{Game, MoveInfo, MoveRecord, color_key};
use gambiit_engine::Stockfish;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use shakmaty::{Color, Role, Square};
use std::{sync::Arc, time::Instant};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    model: String,
    persona: String,
    language: String,
    response_mode: String,
    stockfish_depth: u32,
    multipv: usize,
    theme: String,
    sound: bool,
    animations: bool,
    font_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "opencode-go/deepseek-v4.1-flash".to_string(),
            persona: "Theater Master".to_string(),
            language: "en".to_string(),
            response_mode: "auto".to_string(),
            stockfish_depth: 12,
            multipv: 3,
            theme: "salon".to_string(),
            sound: true,
            animations: true,
            font_scale: 1.2,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PieceView {
    color: String,
    role: String,
    glyph: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SquareView {
    square: String,
    piece: Option<PieceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateView {
    #[serde(rename = "move")]
    mv: String,
    why: Option<String>,
    why_rejected: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommentaryCard {
    san: String,
    uci: String,
    comment: String,
    eval: Option<String>,
    candidates: Vec<CandidateView>,
    plan: Option<String>,
    tokens: Option<u64>,
    cost: Option<f64>,
    elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameSnapshot {
    fen: String,
    turn: String,
    human_color: String,
    agent_color: String,
    status_text: String,
    game_over: bool,
    winner: Option<String>,
    check: bool,
    check_square: Option<String>,
    legal_moves: Vec<MoveInfo>,
    history: Vec<MoveRecord>,
    last_move: Option<MoveRecord>,
    board: Vec<SquareView>,
    commentary: Vec<CommentaryCard>,
    settings: Settings,
    agent_error: Option<String>,
    thinking: bool,
}

struct Session {
    game: Game,
    settings: Settings,
    agent: Option<AgentSession>,
    agent_error: Option<String>,
    stockfish: Option<Stockfish>,
    commentary: Vec<CommentaryCard>,
    thinking: bool,
}

impl Session {
    fn new(mut settings: Settings, human_color: Color) -> Self {
        if settings.model.trim().is_empty() {
            settings.model = Settings::default().model;
        }
        if settings.persona.trim().is_empty() {
            settings.persona = Settings::default().persona;
        }

        let mut game = Game::new();
        game.set_human_color(human_color);

        let mut agent_error = None;
        let agent = match AgentSession::new(AgentConfig {
            model: settings.model.clone(),
            persona: settings.persona.clone(),
            language: settings.language.clone(),
            timeout_secs: 180,
            retries: 1,
        }) {
            Ok(agent) => Some(agent),
            Err(error) => {
                agent_error = Some(format!("Agent unavailable: {error}"));
                None
            }
        };

        let stockfish = Stockfish::discover().and_then(|path| match Stockfish::new(&path) {
            Ok(engine) => Some(engine),
            Err(error) => {
                eprintln!("Stockfish unavailable: {error}");
                None
            }
        });

        Self {
            game,
            settings,
            agent,
            agent_error,
            stockfish,
            commentary: Vec::new(),
            thinking: false,
        }
    }

    fn snapshot(&self) -> GameSnapshot {
        let turn = self.game.turn();
        let mut board = Vec::with_capacity(64);
        let mut check_square = None;

        for rank in (0..8).rev() {
            for file in 0..8 {
                let square = Square::new(rank * 8 + file);
                let piece = self.game.piece_at(square);
                if self.game.is_check()
                    && let Some(piece) = piece
                    && piece.color == turn
                    && piece.role == Role::King
                {
                    check_square = Some(square.to_string());
                }
                board.push(SquareView {
                    square: square.to_string(),
                    piece: piece.map(|piece| PieceView {
                        color: color_key(piece.color).to_string(),
                        role: role_key(piece.role).to_string(),
                        glyph: piece.unicode_char().to_string(),
                    }),
                });
            }
        }

        let status = self.game.status();
        GameSnapshot {
            fen: self.game.fen(),
            turn: color_key(turn).to_string(),
            human_color: color_key(self.game.human_color()).to_string(),
            agent_color: color_key(!self.game.human_color()).to_string(),
            status_text: self.game.status_text(),
            game_over: status.is_over(),
            winner: status.winner().map(|color| color_key(color).to_string()),
            check: self.game.is_check(),
            check_square,
            legal_moves: self.game.legal_moves(),
            history: self.game.history().to_vec(),
            last_move: self.game.last_move().cloned(),
            board,
            commentary: self.commentary.clone(),
            settings: self.settings.clone(),
            agent_error: self.agent_error.clone(),
            thinking: self.thinking,
        }
    }

    fn make_move(&mut self, mv: &str) -> Result<GameSnapshot, String> {
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        if self.game.turn() != self.game.human_color() {
            return Err("It is the agent's turn.".into());
        }
        self.game
            .apply_uci(mv)
            .or_else(|_| self.game.apply_san(mv))
            .map_err(|error| error.to_string())?;
        Ok(self.snapshot())
    }

    fn agent_move(&mut self) -> Result<GameSnapshot, String> {
        if self.thinking {
            return Err("The agent is already thinking.".into());
        }
        self.thinking = true;
        let result = self.agent_move_inner();
        self.thinking = false;
        result
    }

    fn agent_move_inner(&mut self) -> Result<GameSnapshot, String> {
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        if self.game.turn() == self.game.human_color() {
            return Err("It is the human's turn.".into());
        }

        let candidates = match self.stockfish.as_mut() {
            Some(engine) => match engine.candidates(
                &self.game.fen(),
                self.settings.multipv.max(1),
                self.settings.stockfish_depth.max(1),
            ) {
                Ok(candidates) => candidates,
                Err(error) => {
                    eprintln!("Stockfish warning: {error}");
                    Vec::new()
                }
            },
            None => Vec::new(),
        };

        let prompt_game = self.game.clone();
        let validation_game = prompt_game.clone();
        let legal_hint = prompt_game.legal_uci().join(", ");
        let validate = move |turn: &AgentTurn| {
            let mut probe = validation_game.clone();
            if probe.apply_uci(&turn.mv).is_ok() {
                return Ok(());
            }
            if probe.apply_san(&turn.mv).is_ok() {
                return Ok(());
            }
            Err(format!(
                "'{}' is not a legal move. Legal UCI moves: {}",
                turn.mv, legal_hint
            ))
        };

        let started = Instant::now();
        let result = if let Some(agent) = self.agent.as_mut() {
            agent
                .request_move(&prompt_game, &candidates, validate)
                .map_err(|error| error.to_string())
        } else {
            Err(self
                .agent_error
                .clone()
                .unwrap_or_else(|| "Agent is unavailable.".into()))
        };

        let (turn, output) = match result {
            Ok(value) => value,
            Err(error) => {
                self.agent_error = Some(error.clone());
                return Err(error);
            }
        };

        let info = match self
            .game
            .apply_uci(&turn.mv)
            .or_else(|_| self.game.apply_san(&turn.mv))
        {
            Ok(info) => info,
            Err(error) => {
                let error = error.to_string();
                self.agent_error = Some(error.clone());
                return Err(error);
            }
        };

        self.game
            .annotate_last(Some(turn.comment.clone()), turn.eval.clone());
        let candidates = turn
            .candidates
            .iter()
            .map(|candidate| CandidateView {
                mv: candidate.mv.clone(),
                why: candidate.why.clone(),
                why_rejected: candidate.why_rejected.clone(),
            })
            .collect();
        self.commentary.push(CommentaryCard {
            san: info.san.clone(),
            uci: info.uci.clone(),
            comment: turn.comment.clone(),
            eval: turn.eval.clone(),
            candidates,
            plan: turn.plan.clone(),
            tokens: output.tokens,
            cost: output.cost,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
        self.agent_error = None;
        Ok(self.snapshot())
    }

    fn takeback(&mut self) -> Result<GameSnapshot, String> {
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        let last = self
            .game
            .last_move()
            .cloned()
            .ok_or_else(|| "There is no move to take back.".to_string())?;
        if last.color != color_key(self.game.human_color()) {
            return Err("Only your own last move can be taken back.".into());
        }
        self.game.undo_last();
        Ok(self.snapshot())
    }

    fn resign(&mut self) -> Result<GameSnapshot, String> {
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        self.game.resign(!self.game.human_color());
        Ok(self.snapshot())
    }
}

struct AppState {
    session: Arc<Mutex<Session>>,
}

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    Ok(state.session.lock().snapshot())
}

#[tauri::command]
fn new_game(
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
fn make_move(state: State<'_, AppState>, mv: String) -> Result<GameSnapshot, String> {
    state.session.lock().make_move(&mv)
}

#[tauri::command]
async fn agent_move(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    let session = state.session.clone();
    tauri::async_runtime::spawn_blocking(move || session.lock().agent_move())
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
fn takeback(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().takeback()
}

#[tauri::command]
fn resign(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().resign()
}

#[tauri::command]
fn list_models() -> Result<Vec<String>, String> {
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

fn parse_color(value: &str) -> Option<Color> {
    match value.to_ascii_lowercase().as_str() {
        "white" | "w" => Some(Color::White),
        "black" | "b" => Some(Color::Black),
        _ => None,
    }
}

fn role_key(role: Role) -> &'static str {
    match role {
        Role::Pawn => "p",
        Role::Knight => "n",
        Role::Bishop => "b",
        Role::Rook => "r",
        Role::Queen => "q",
        Role::King => "k",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            session: Arc::new(Mutex::new(Session::new(Settings::default(), Color::White))),
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            new_game,
            make_move,
            agent_move,
            takeback,
            resign,
            list_models
        ])
        .run(tauri::generate_context!())
        .expect("error while running Gambiit");
}

fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(Settings::default(), Color::White)
    }

    #[test]
    fn snapshot_starts_from_the_initial_position() {
        let session = session();
        let snapshot = session.snapshot();
        assert_eq!(snapshot.turn, "white");
        assert_eq!(snapshot.human_color, "white");
        assert_eq!(snapshot.agent_color, "black");
        assert_eq!(snapshot.board.len(), 64);
        assert_eq!(snapshot.legal_moves.len(), 20);
        assert_eq!(snapshot.history.len(), 0);
        assert!(!snapshot.game_over);
    }

    #[test]
    fn human_move_and_takeback_work() {
        let mut session = session();
        let snapshot = session.make_move("e2e4").unwrap();
        assert_eq!(snapshot.turn, "black");
        assert_eq!(snapshot.history.len(), 1);
        assert_eq!(snapshot.history[0].san, "e4");
        assert_eq!(snapshot.history[0].color, "white");

        let snapshot = session.takeback().unwrap();
        assert_eq!(snapshot.turn, "white");
        assert_eq!(snapshot.history.len(), 0);
        assert_eq!(snapshot.legal_moves.len(), 20);
    }

    #[test]
    fn human_cannot_move_on_the_agent_turn() {
        let mut session = session();
        session.make_move("e2e4").unwrap();
        assert!(session.make_move("e7e5").is_err());
    }

    #[test]
    fn resign_awards_the_game_to_the_agent() {
        let mut session = session();
        let snapshot = session.resign().unwrap();
        assert!(snapshot.game_over);
        assert_eq!(snapshot.winner.as_deref(), Some("black"));
        assert!(session.takeback().is_err());
    }

    #[test]
    fn playing_as_black_starts_with_the_agent_to_move() {
        let session = Session::new(Settings::default(), Color::Black);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.human_color, "black");
        assert_eq!(snapshot.agent_color, "white");
        assert_eq!(snapshot.turn, "white");
    }

    /// End-to-end smoke test against the real configured OpenCode model.
    /// Ignored by default because it spends provider tokens and takes seconds.
    /// Run with:
    ///   cargo test -p gambiit-app -- --ignored agent_smoke --nocapture
    #[test]
    #[ignore]
    fn agent_smoke() {
        let mut session = session();
        session.make_move("e2e4").unwrap();
        let snapshot = session.agent_move().unwrap();
        assert_eq!(snapshot.history.len(), 2);
        assert_eq!(snapshot.history[1].color, "black");
        assert_eq!(snapshot.commentary.len(), 1);
        assert!(!snapshot.commentary[0].comment.is_empty());
        assert_eq!(snapshot.turn, "white");
        eprintln!(
            "agent replied {} in {} ms; {} tokens",
            snapshot.history[1].san,
            snapshot.commentary[0].elapsed_ms,
            snapshot.commentary[0].tokens.unwrap_or(0)
        );
    }
}
