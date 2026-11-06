#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis;
mod bluff;
mod coach;
mod credentials;
mod director;
mod leaderboard;
mod mcp;
mod pgn;
mod pieces;
mod storage;
mod style;

use gambiit_agent::{AgentConfig, AgentSession, AgentTurn, DrawDecision, OpenCodeClient};
use gambiit_core::{Game, GameStatus, MoveInfo, MoveRecord, color_key};
use gambiit_engine::{Candidate, Stockfish};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use shakmaty::{Color, Role, Square};
use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct CustomTheme {
    bg: String,
    panel: String,
    border: String,
    text: String,
    muted: String,
    accent: String,
    board_light: String,
    board_dark: String,
    board_border: String,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            bg: "#14110e".to_string(),
            panel: "#201a15".to_string(),
            border: "#3d3126".to_string(),
            text: "#efe6d5".to_string(),
            muted: "#a89a86".to_string(),
            accent: "#c9a227".to_string(),
            board_light: "#e8d3b0".to_string(),
            board_dark: "#8b5a2b".to_string(),
            board_border: "#3b2a1a".to_string(),
        }
    }
}

fn default_piece_set() -> String {
    "cburnett".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct CustomPersona {
    name: String,
    accent: String,
    theatricality: u8,
    confidence: u8,
    warmth: u8,
    humor: u8,
    aggression: u8,
    verbosity: u8,
    deception: u8,
}

impl Default for CustomPersona {
    fn default() -> Self {
        Self {
            name: "Custom Persona".to_string(),
            accent: "#c9a227".to_string(),
            theatricality: 7,
            confidence: 6,
            warmth: 5,
            humor: 5,
            aggression: 5,
            verbosity: 5,
            deception: 4,
        }
    }
}

impl Settings {
    fn persona_name(&self) -> String {
        if self.persona == "Custom" && !self.custom_persona.name.trim().is_empty() {
            self.custom_persona.name.trim().to_string()
        } else {
            self.persona.clone()
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_agent_mode() -> String {
    "prompt".to_string()
}

fn default_persona_style() -> String {
    "subtle".to_string()
}

fn default_time_control() -> String {
    "rapid".to_string()
}

fn default_clock_initial() -> u64 {
    600_000
}

fn default_clock_increment() -> u64 {
    5_000
}

fn default_bluff_intensity() -> u8 {
    50
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    model: String,
    persona: String,
    ui_language: String,
    agent_language: String,
    agent_language_name: String,
    response_mode: String,
    #[serde(default = "default_agent_mode")]
    agent_mode: String,
    #[serde(default = "default_persona_style")]
    persona_style: String,
    #[serde(default)]
    custom_persona: CustomPersona,
    #[serde(default = "default_bluff_intensity")]
    bluff_intensity: u8,
    stockfish_depth: u32,
    multipv: usize,
    difficulty: String,
    theme: String,
    #[serde(default)]
    custom_theme: CustomTheme,
    #[serde(default = "default_piece_set")]
    piece_set: String,
    #[serde(default = "default_true")]
    chat_enabled: bool,
    #[serde(default = "default_true")]
    coach_enabled: bool,
    #[serde(default = "default_true")]
    draw_offers_enabled: bool,
    #[serde(default)]
    clock_enabled: bool,
    #[serde(default = "default_time_control")]
    time_control: String,
    #[serde(default = "default_clock_initial")]
    clock_initial_ms: u64,
    #[serde(default = "default_clock_increment")]
    clock_increment_ms: u64,
    sound: bool,
    animations: bool,
    font_scale: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "opencode-go/deepseek-v4.1-flash".to_string(),
            persona: "Theater Master".to_string(),
            ui_language: "en".to_string(),
            agent_language: "en".to_string(),
            agent_language_name: "English".to_string(),
            response_mode: "auto".to_string(),
            agent_mode: default_agent_mode(),
            persona_style: default_persona_style(),
            custom_persona: CustomPersona::default(),
            bluff_intensity: default_bluff_intensity(),
            stockfish_depth: 11,
            multipv: 4,
            difficulty: "intermediate".to_string(),
            theme: "salon".to_string(),
            custom_theme: CustomTheme::default(),
            piece_set: default_piece_set(),
            chat_enabled: true,
            coach_enabled: true,
            draw_offers_enabled: true,
            clock_enabled: false,
            time_control: default_time_control(),
            clock_initial_ms: default_clock_initial(),
            clock_increment_ms: default_clock_increment(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CandidateView {
    #[serde(rename = "move")]
    mv: String,
    why: Option<String>,
    why_rejected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommentaryCard {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatMessage {
    role: String,
    text: String,
    at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoachMessage {
    action: String,
    level: Option<u8>,
    text: String,
    model: String,
    tokens: Option<u64>,
    cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClockView {
    white_ms: u64,
    black_ms: u64,
    increment_ms: u64,
    running: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedClock {
    pub white_ms: u64,
    pub black_ms: u64,
    pub increment_ms: u64,
}

struct GameClock {
    white_ms: u64,
    black_ms: u64,
    increment_ms: u64,
    last_update: Instant,
}

impl GameClock {
    fn from_settings(settings: &Settings) -> Option<Self> {
        settings.clock_enabled.then(|| Self {
            white_ms: settings.clock_initial_ms,
            black_ms: settings.clock_initial_ms,
            increment_ms: settings.clock_increment_ms,
            last_update: Instant::now(),
        })
    }

    fn from_saved(saved: SavedClock) -> Self {
        Self {
            white_ms: saved.white_ms,
            black_ms: saved.black_ms,
            increment_ms: saved.increment_ms,
            last_update: Instant::now(),
        }
    }

    fn view(&self, running: Option<Color>) -> ClockView {
        ClockView {
            white_ms: self.white_ms,
            black_ms: self.black_ms,
            increment_ms: self.increment_ms,
            running: running.map(|color| color_key(color).to_string()),
        }
    }

    fn tick(&mut self, turn: Color, elapsed_ms: u64) -> bool {
        if elapsed_ms == 0 {
            return false;
        }
        let remaining = match turn {
            Color::White => &mut self.white_ms,
            Color::Black => &mut self.black_ms,
        };
        if *remaining <= elapsed_ms {
            *remaining = 0;
            true
        } else {
            *remaining -= elapsed_ms;
            false
        }
    }

    fn add_increment(&mut self, color: Color) {
        let remaining = match color {
            Color::White => &mut self.white_ms,
            Color::Black => &mut self.black_ms,
        };
        *remaining = remaining.saturating_add(self.increment_ms);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClockUpdate {
    game_id: String,
    clock: Option<ClockView>,
    game_over: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DrawOfferOutcome {
    accepted: bool,
    comment: String,
    snapshot: GameSnapshot,
}

#[derive(Clone, Debug)]
struct CoachJob {
    game_id: String,
    fen: String,
    history_len: usize,
    action: String,
    level: Option<u8>,
    prompt: String,
    config: AgentConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameSnapshot {
    game_id: String,
    game_name: String,
    fen: String,
    turn: String,
    human_color: String,
    agent_color: String,
    status_text: String,
    status_key: String,
    status_side: Option<String>,
    game_over: bool,
    winner: Option<String>,
    check: bool,
    check_square: Option<String>,
    legal_moves: Vec<MoveInfo>,
    history: Vec<MoveRecord>,
    last_move: Option<MoveRecord>,
    board: Vec<SquareView>,
    commentary: Vec<CommentaryCard>,
    chat: Vec<ChatMessage>,
    coach: Vec<CoachMessage>,
    coach_thinking: bool,
    clock: Option<ClockView>,
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
    chat: Vec<ChatMessage>,
    coach: Vec<CoachMessage>,
    coach_thinking: bool,
    clock: Option<GameClock>,
    analysis: Option<analysis::AnalysisSummary>,
    thinking: bool,
    game_id: String,
    game_name: String,
    created_at: u64,
    persist_enabled: bool,
}

impl Session {
    fn new(settings: Settings, human_color: Color) -> Self {
        Self::new_with_persist(settings, human_color, true)
    }

    fn new_with_persist(settings: Settings, human_color: Color, persist_enabled: bool) -> Self {
        let settings = normalize_settings(settings);
        let mut game = Game::new();
        game.set_human_color(human_color);
        let game_id = storage::new_id();
        let game_name = format!("Game vs {}", settings.persona_name());
        let clock = GameClock::from_settings(&settings);
        let mut session = Self::assemble(
            game,
            settings,
            Vec::new(),
            Vec::new(),
            clock,
            None,
            game_id,
            game_name,
            storage::now(),
        );
        session.persist_enabled = persist_enabled;
        if persist_enabled {
            session.persist();
        }
        session
    }

    fn from_saved(saved: storage::SavedGame) -> Result<Self, String> {
        let settings = normalize_settings(saved.settings);
        let mut game = if saved.initial_fen.trim().is_empty() {
            Game::new()
        } else {
            Game::from_fen(&saved.initial_fen).map_err(|error| error.to_string())?
        };
        for record in &saved.moves {
            game.apply_uci(&record.uci)
                .map_err(|error| format!("Could not replay {}: {error}", record.uci))?;
            game.annotate_last(record.comment.clone(), record.eval.clone());
        }
        if let Some(color) = parse_color(&saved.human_color) {
            game.set_human_color(color);
        }
        match saved.termination.as_str() {
            "resigned_white" => game.resign(Color::Black),
            "resigned_black" => game.resign(Color::White),
            "timeout_white" => game.timeout(Color::White),
            "timeout_black" => game.timeout(Color::Black),
            "draw_agreed" => game.draw_agreed(),
            "aborted" => game.abort(),
            _ => {}
        }
        let clock = if settings.clock_enabled {
            saved
                .clock
                .map(GameClock::from_saved)
                .or_else(|| GameClock::from_settings(&settings))
        } else {
            None
        };
        Ok(Self::assemble(
            game,
            settings,
            saved.commentary,
            saved.chat,
            clock,
            saved.analysis,
            saved.id,
            saved.name,
            saved.created_at,
        ))
    }

    fn from_import(
        game: Game,
        settings: Settings,
        commentary: Vec<CommentaryCard>,
        name: String,
    ) -> Self {
        let settings = normalize_settings(settings);
        let clock = GameClock::from_settings(&settings);
        let session = Self::assemble(
            game,
            settings,
            commentary,
            Vec::new(),
            clock,
            None,
            storage::new_id(),
            name,
            storage::now(),
        );
        session.persist();
        session
    }

    #[allow(clippy::too_many_arguments)]
    fn assemble(
        game: Game,
        settings: Settings,
        commentary: Vec<CommentaryCard>,
        chat: Vec<ChatMessage>,
        clock: Option<GameClock>,
        analysis: Option<analysis::AnalysisSummary>,
        game_id: String,
        game_name: String,
        created_at: u64,
    ) -> Self {
        let mut agent_error = None;
        let credential_env = credential_env_for_model(&settings.model);
        let agent = match AgentSession::new(agent_config(&settings, credential_env)) {
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
            commentary,
            chat,
            coach: Vec::new(),
            coach_thinking: false,
            clock,
            analysis,
            thinking: false,
            game_id,
            game_name,
            created_at,
            persist_enabled: true,
        }
    }

    fn saved(&self) -> storage::SavedGame {
        let termination = match self.game.status() {
            GameStatus::Resigned {
                winner: Color::White,
            } => "resigned_black",
            GameStatus::Resigned { .. } => "resigned_white",
            GameStatus::Timeout {
                winner: Color::White,
            } => "timeout_black",
            GameStatus::Timeout { .. } => "timeout_white",
            GameStatus::Draw {
                reason: "agreement",
            } => "draw_agreed",
            GameStatus::Aborted => "aborted",
            _ => "ongoing",
        };
        storage::SavedGame {
            id: self.game_id.clone(),
            name: self.game_name.clone(),
            created_at: self.created_at,
            updated_at: storage::now(),
            human_color: color_key(self.game.human_color()).to_string(),
            initial_fen: self.game.initial_fen().to_string(),
            moves: self.game.history().to_vec(),
            commentary: self.commentary.clone(),
            chat: self.chat.clone(),
            clock: self.clock.as_ref().map(|clock| SavedClock {
                white_ms: clock.white_ms,
                black_ms: clock.black_ms,
                increment_ms: clock.increment_ms,
            }),
            analysis: self.analysis.clone(),
            settings: self.settings.clone(),
            result: pgn::result(&self.game).to_string(),
            termination: termination.to_string(),
        }
    }

    fn persist(&self) {
        if !self.persist_enabled {
            return;
        }
        if let Err(error) = storage::save(&self.saved()) {
            eprintln!("Could not save game: {error}");
        }
    }

    fn update_settings(&mut self, next: Settings) -> Result<GameSnapshot, String> {
        self.settings = normalize_settings(next);
        let credential_env = credential_env_for_model(&self.settings.model);
        let config = agent_config(&self.settings, credential_env);
        if let Some(agent) = self.agent.as_mut() {
            agent.update_config(config);
            self.agent_error = None;
        } else {
            match AgentSession::new(config) {
                Ok(agent) => {
                    self.agent = Some(agent);
                    self.agent_error = None;
                }
                Err(error) => {
                    self.agent_error = Some(format!("Agent unavailable: {error}"));
                }
            }
        }
        self.persist();
        Ok(self.snapshot())
    }

    fn prepare_coach(&mut self, action: &str, level: Option<u8>) -> Result<CoachJob, String> {
        if !self.settings.coach_enabled {
            return Err("Coach is disabled in settings.".to_string());
        }
        if self.coach_thinking {
            return Err("The coach is already thinking.".to_string());
        }
        if self.game.status().is_over() {
            return Err("The game is over.".to_string());
        }

        let game = self.game.clone();
        let human_color = game.human_color();
        let language = self.settings.agent_language_name.clone();

        let (prompt, level) = match action {
            "explain" => {
                let (eval_text, _) = {
                    let engine = self.stockfish.as_mut().ok_or_else(|| {
                        "The coach needs Stockfish. Install it to get honest analysis.".to_string()
                    })?;
                    coach_context(engine, &game, 1)?
                };
                (
                    coach::explain_prompt(&game, human_color, &eval_text, &language),
                    None,
                )
            }
            "hint" => {
                if game.turn() != human_color {
                    return Err("Hints are available on your turn.".to_string());
                }
                let level = level.unwrap_or(1).clamp(1, 3);
                let (eval_text, candidates) = {
                    let engine = self.stockfish.as_mut().ok_or_else(|| {
                        "The coach needs Stockfish. Install it to get honest hints.".to_string()
                    })?;
                    coach_context(engine, &game, 3)?
                };
                (
                    coach::hint_prompt(
                        &game,
                        human_color,
                        level,
                        &candidates,
                        &eval_text,
                        &language,
                    ),
                    Some(level),
                )
            }
            "review" => {
                let facts = {
                    let engine = self.stockfish.as_mut().ok_or_else(|| {
                        "The coach needs Stockfish. Install it to review moves.".to_string()
                    })?;
                    coach::review_facts(engine, &game, human_color)?
                };
                (coach::review_prompt(&facts, &language), None)
            }
            _ => return Err("Unknown coach action.".to_string()),
        };

        let config = coach_agent_config(
            &self.settings,
            credential_env_for_model(&self.settings.model),
        );
        self.coach_thinking = true;
        Ok(CoachJob {
            game_id: self.game_id.clone(),
            fen: self.game.fen(),
            history_len: self.game.history().len(),
            action: action.to_string(),
            level,
            prompt,
            config,
        })
    }

    fn finish_coach(
        &mut self,
        job: &CoachJob,
        result: Result<(String, Option<u64>, Option<f64>), String>,
    ) -> Result<GameSnapshot, String> {
        self.coach_thinking = false;
        let (text, tokens, cost) = result?;
        if !self.settings.coach_enabled {
            return Err("Coach is disabled in settings.".to_string());
        }
        if job.game_id != self.game_id
            || job.fen != self.game.fen()
            || job.history_len != self.game.history().len()
        {
            return Err(
                "The position changed while the coach was thinking. Ask again.".to_string(),
            );
        }
        self.coach.push(CoachMessage {
            action: job.action.clone(),
            level: job.level,
            text,
            model: job.config.model.clone(),
            tokens,
            cost,
        });
        if self.coach.len() > 40 {
            self.coach.remove(0);
        }
        Ok(self.snapshot())
    }

    fn clear_coach(&mut self) -> Result<GameSnapshot, String> {
        if !self.settings.coach_enabled {
            return Err("Coach is disabled in settings.".to_string());
        }
        if self.coach_thinking {
            return Err("The coach is still thinking.".to_string());
        }
        self.coach.clear();
        Ok(self.snapshot())
    }

    #[cfg(test)]
    fn chat_send(&mut self, message: &str) -> Result<GameSnapshot, String> {
        self.chat_send_with_mcp(message, None)
    }

    fn chat_send_with_mcp(
        &mut self,
        message: &str,
        bridge: Option<Arc<mcp::Bridge>>,
    ) -> Result<GameSnapshot, String> {
        if !self.settings.chat_enabled {
            return Err("Opponent chat is disabled in settings.".to_string());
        }
        let text = message.trim();
        if text.is_empty() {
            return Err("The message is empty.".to_string());
        }
        if text.chars().count() > 2_000 {
            return Err("The message is too long (maximum 2000 characters).".to_string());
        }

        let history: Vec<(String, String)> = self
            .chat
            .iter()
            .filter(|entry| entry.role != "system")
            .map(|entry| (entry.role.clone(), entry.text.clone()))
            .collect();
        let use_mcp = self.settings.agent_mode == "mcp" && bridge.is_some();
        let result = if use_mcp {
            self.request_mcp_chat(text, bridge.as_ref().expect("checked above"))
                .map(|(reply, _)| reply)
        } else if let Some(agent) = self.agent.as_mut() {
            agent
                .request_chat(&self.game, &history, text)
                .map(|(reply, _)| reply)
                .map_err(|error| error.to_string())
        } else {
            Err(self
                .agent_error
                .clone()
                .unwrap_or_else(|| "Agent is unavailable.".to_string()))
        };

        self.chat.push(ChatMessage {
            role: "user".to_string(),
            text: text.to_string(),
            at: storage::now(),
        });
        match result {
            Ok(reply) => {
                self.chat.push(ChatMessage {
                    role: "agent".to_string(),
                    text: reply,
                    at: storage::now(),
                });
            }
            Err(error) => {
                self.chat.push(ChatMessage {
                    role: "system".to_string(),
                    text: format!("Chat unavailable: {error}"),
                    at: storage::now(),
                });
            }
        }
        self.persist();
        Ok(self.snapshot())
    }

    fn clear_chat(&mut self) -> Result<GameSnapshot, String> {
        self.chat.clear();
        self.persist();
        Ok(self.snapshot())
    }

    #[cfg(test)]
    fn offer_draw(&mut self) -> Result<DrawOfferOutcome, String> {
        self.offer_draw_with_mcp(None)
    }

    fn offer_draw_with_mcp(
        &mut self,
        bridge: Option<Arc<mcp::Bridge>>,
    ) -> Result<DrawOfferOutcome, String> {
        self.tick_clock();
        if !self.settings.draw_offers_enabled {
            return Err("Draw offers are disabled in settings.".to_string());
        }
        if self.game.status().is_over() {
            return Err("The game is over.".to_string());
        }

        let use_mcp = self.settings.agent_mode == "mcp" && bridge.is_some();
        let result = if use_mcp {
            self.request_mcp_draw(bridge.as_ref().expect("checked above"))
        } else if let Some(agent) = self.agent.as_mut() {
            agent
                .request_draw_decision(&self.game)
                .map(|(decision, _)| decision)
                .map_err(|error| error.to_string())
        } else {
            Err(self
                .agent_error
                .clone()
                .unwrap_or_else(|| "Agent is unavailable.".to_string()))
        };
        let decision = result?;
        if decision.accept {
            self.game.draw_agreed();
        }
        let comment = if decision.comment.trim().is_empty() {
            if decision.accept {
                "A draw, then.".to_string()
            } else {
                "I decline the draw.".to_string()
            }
        } else {
            decision.comment.trim().chars().take(800).collect()
        };
        if self.settings.chat_enabled {
            self.chat.push(ChatMessage {
                role: "agent".to_string(),
                text: comment.clone(),
                at: storage::now(),
            });
        }
        self.persist();
        Ok(DrawOfferOutcome {
            accepted: decision.accept,
            comment,
            snapshot: self.snapshot(),
        })
    }

    fn tick_clock(&mut self) {
        let turn = self.game.turn();
        if self.game.status().is_over() {
            if let Some(clock) = self.clock.as_mut() {
                clock.last_update = Instant::now();
            }
            return;
        }
        let flagged = {
            let Some(clock) = self.clock.as_mut() else {
                return;
            };
            let now = Instant::now();
            let elapsed = now
                .duration_since(clock.last_update)
                .as_millis()
                .min(u64::MAX as u128) as u64;
            clock.last_update = now;
            clock.tick(turn, elapsed)
        };
        if flagged {
            self.game.timeout(turn);
            self.persist();
        }
    }

    fn add_increment(&mut self, color: Color) {
        if let Some(clock) = self.clock.as_mut() {
            clock.add_increment(color);
            clock.last_update = Instant::now();
        }
    }

    fn clock_view(&self) -> Option<ClockView> {
        let clock = self.clock.as_ref()?;
        let running = if self.game.status().is_over() {
            None
        } else {
            Some(self.game.turn())
        };
        Some(clock.view(running))
    }

    fn snapshot(&mut self) -> GameSnapshot {
        self.tick_clock();
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
        let (status_key, status_side) = status_parts(status, turn);
        GameSnapshot {
            game_id: self.game_id.clone(),
            game_name: self.game_name.clone(),
            fen: self.game.fen(),
            turn: color_key(turn).to_string(),
            human_color: color_key(self.game.human_color()).to_string(),
            agent_color: color_key(!self.game.human_color()).to_string(),
            status_text: self.game.status_text(),
            status_key,
            status_side,
            game_over: status.is_over(),
            winner: status.winner().map(|color| color_key(color).to_string()),
            check: self.game.is_check(),
            check_square,
            legal_moves: self.game.legal_moves(),
            history: self.game.history().to_vec(),
            last_move: self.game.last_move().cloned(),
            board,
            commentary: self.commentary.clone(),
            chat: self.chat.clone(),
            coach: self.coach.clone(),
            coach_thinking: self.coach_thinking,
            clock: self.clock_view(),
            settings: self.settings.clone(),
            agent_error: self.agent_error.clone(),
            thinking: self.thinking,
        }
    }

    fn make_move(&mut self, mv: &str) -> Result<GameSnapshot, String> {
        self.tick_clock();
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        if self.game.turn() != self.game.human_color() {
            return Err("It is the agent's turn.".into());
        }
        let mover = self.game.turn();
        self.game
            .apply_uci(mv)
            .or_else(|_| self.game.apply_san(mv))
            .map_err(|error| error.to_string())?;
        self.analysis = None;
        self.add_increment(mover);
        self.persist();
        Ok(self.snapshot())
    }

    #[cfg(test)]
    fn agent_move(&mut self) -> Result<GameSnapshot, String> {
        self.agent_move_with_mcp(None)
    }

    fn agent_move_with_mcp(
        &mut self,
        bridge: Option<Arc<mcp::Bridge>>,
    ) -> Result<GameSnapshot, String> {
        if self.thinking {
            return Err("The agent is already thinking.".into());
        }
        self.thinking = true;
        let result = self.agent_move_inner(bridge);
        self.thinking = false;
        result
    }

    fn agent_move_inner(
        &mut self,
        bridge: Option<Arc<mcp::Bridge>>,
    ) -> Result<GameSnapshot, String> {
        self.tick_clock();
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        if self.game.turn() == self.game.human_color() {
            return Err("It is the human's turn.".into());
        }

        let (multipv, depth, skill) = engine_params(&self.settings);
        let mut candidates = match self.stockfish.as_mut() {
            Some(engine) => {
                if let Err(error) = engine.set_skill(skill) {
                    eprintln!("Stockfish skill warning: {error}");
                }
                match engine.candidates(&self.game.fen(), multipv, depth) {
                    Ok(candidates) => candidates,
                    Err(error) => {
                        eprintln!("Stockfish warning: {error}");
                        Vec::new()
                    }
                }
            }
            None => Vec::new(),
        };

        let prompt_game = self.game.clone();
        let tolerance = style::tolerance(&self.settings.persona_style);
        let style_index = if self.settings.persona == "Custom" {
            style::choose_custom(
                &prompt_game,
                &candidates,
                &self.settings.custom_persona,
                tolerance,
            )
        } else {
            style::choose(&prompt_game, &candidates, &self.settings.persona, tolerance)
        };
        if let Some(index) = style_index
            && index > 0
        {
            candidates.swap(0, index);
        }
        let validation_game = prompt_game.clone();
        let legal_hint = prompt_game.legal_uci().join(", ");
        let candidate_ucis: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.uci.clone())
            .collect();
        let validate = move |turn: &AgentTurn| {
            let mut probe = validation_game.clone();
            let info = probe
                .apply_uci(&turn.mv)
                .or_else(|_| probe.apply_san(&turn.mv))
                .map_err(|error| {
                    format!(
                        "'{}' is not a legal move ({error}). Legal UCI moves: {}",
                        turn.mv, legal_hint
                    )
                })?;
            if !candidate_ucis.is_empty() && !candidate_ucis.contains(&info.uci) {
                return Err(format!(
                    "You must choose one of the Stockfish candidate moves: {}.",
                    candidate_ucis.join(", ")
                ));
            }
            Ok(())
        };

        let started = Instant::now();
        let use_mcp = self.settings.agent_mode == "mcp" && bridge.is_some();
        let result = if use_mcp {
            self.request_mcp_turn(
                &prompt_game,
                &candidates,
                bridge.as_ref().expect("checked above"),
            )
        } else if let Some(agent) = self.agent.as_mut() {
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

        self.tick_clock();
        if self.game.status().is_over() {
            return Err("Time ran out while the agent was thinking.".into());
        }
        let mover = self.game.turn();
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
        self.analysis = None;
        let mut candidates: Vec<CandidateView> = turn
            .candidates
            .iter()
            .map(|candidate| CandidateView {
                mv: candidate.mv.clone(),
                why: candidate.why.clone(),
                why_rejected: candidate.why_rejected.clone(),
            })
            .collect();
        shuffle_candidates(&mut candidates);
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
        self.add_increment(mover);
        self.persist();
        Ok(self.snapshot())
    }

    fn analysis_value(&self) -> Option<serde_json::Value> {
        self.analysis
            .as_ref()
            .and_then(|summary| serde_json::to_value(summary).ok())
    }

    fn mcp_candidates(&mut self) -> Vec<Candidate> {
        let (multipv, depth, skill) = engine_params(&self.settings);
        let fen = self.game.fen();
        let Some(engine) = self.stockfish.as_mut() else {
            return Vec::new();
        };
        if let Err(error) = engine.set_skill(skill) {
            eprintln!("Stockfish skill warning: {error}");
        }
        engine.candidates(&fen, multipv, depth).unwrap_or_default()
    }

    fn begin_mcp_turn(
        &self,
        kind: mcp::TurnKind,
        candidates: Vec<Candidate>,
        bridge: &Arc<mcp::Bridge>,
    ) {
        let clock = serde_json::to_value(self.clock_view()).ok();
        bridge.begin(mcp::TurnContext {
            kind,
            game: self.game.clone(),
            legal: self.game.legal_moves(),
            candidates,
            clock,
            analysis: self.analysis_value(),
        });
    }

    fn request_mcp_turn(
        &mut self,
        game: &Game,
        candidates: &[gambiit_engine::Candidate],
        bridge: &Arc<mcp::Bridge>,
    ) -> Result<(AgentTurn, gambiit_agent::OpenCodeOutput), String> {
        self.begin_mcp_turn(mcp::TurnKind::Move, candidates.to_vec(), bridge);
        let config_path = match write_mcp_config(bridge) {
            Ok(path) => path,
            Err(error) => {
                bridge.clear();
                return Err(error);
            }
        };
        let config = agent_config(
            &self.settings,
            credential_env_for_model(&self.settings.model),
        );
        let prompt = gambiit_agent::build_mcp_move_prompt(game, &config, candidates);
        let result = if let Some(agent) = self.agent.as_mut() {
            agent
                .request_mcp(&prompt, &config_path)
                .map_err(|error| error.to_string())
        } else {
            Err(self
                .agent_error
                .clone()
                .unwrap_or_else(|| "Agent is unavailable.".into()))
        };
        let submission = bridge.take_submission();
        bridge.clear();
        let output = result?;
        let submission = match submission {
            Some(mcp::Submission::Move(submission)) => submission,
            Some(_) => {
                return Err("The agent submitted the wrong response type for a move turn.".into());
            }
            None => return Err("The agent did not call submit_move.".into()),
        };
        Ok((
            AgentTurn {
                mv: submission.mv,
                comment: submission.comment,
                eval: submission.eval,
                candidates: submission.candidates,
                plan: submission.plan,
            },
            output,
        ))
    }

    fn request_mcp_chat(
        &mut self,
        message: &str,
        bridge: &Arc<mcp::Bridge>,
    ) -> Result<(String, gambiit_agent::OpenCodeOutput), String> {
        let candidates = self.mcp_candidates();
        self.begin_mcp_turn(mcp::TurnKind::Chat, candidates, bridge);
        let config_path = match write_mcp_config(bridge) {
            Ok(path) => path,
            Err(error) => {
                bridge.clear();
                return Err(error);
            }
        };
        let config = agent_config(
            &self.settings,
            credential_env_for_model(&self.settings.model),
        );
        let history: Vec<(String, String)> = self
            .chat
            .iter()
            .filter(|entry| entry.role != "system")
            .map(|entry| (entry.role.clone(), entry.text.clone()))
            .collect();
        let prompt = gambiit_agent::build_mcp_chat_prompt(&self.game, &config, &history, message);
        let result = if let Some(agent) = self.agent.as_mut() {
            agent
                .request_mcp(&prompt, &config_path)
                .map_err(|error| error.to_string())
        } else {
            Err(self
                .agent_error
                .clone()
                .unwrap_or_else(|| "Agent is unavailable.".into()))
        };
        let submission = bridge.take_submission();
        bridge.clear();
        let output = result?;
        match submission {
            Some(mcp::Submission::Chat(chat)) => Ok((chat.message, output)),
            Some(_) => Err("The agent submitted the wrong response type for a chat turn.".into()),
            None => Err("The agent did not call submit_chat.".into()),
        }
    }

    fn request_mcp_draw(&mut self, bridge: &Arc<mcp::Bridge>) -> Result<DrawDecision, String> {
        let candidates = self.mcp_candidates();
        self.begin_mcp_turn(mcp::TurnKind::Draw, candidates, bridge);
        let config_path = match write_mcp_config(bridge) {
            Ok(path) => path,
            Err(error) => {
                bridge.clear();
                return Err(error);
            }
        };
        let config = agent_config(
            &self.settings,
            credential_env_for_model(&self.settings.model),
        );
        let prompt = gambiit_agent::build_mcp_draw_prompt(&self.game, &config);
        let result = if let Some(agent) = self.agent.as_mut() {
            agent
                .request_mcp(&prompt, &config_path)
                .map_err(|error| error.to_string())
        } else {
            Err(self
                .agent_error
                .clone()
                .unwrap_or_else(|| "Agent is unavailable.".into()))
        };
        let submission = bridge.take_submission();
        bridge.clear();
        result?;
        match submission {
            Some(mcp::Submission::Draw(draw)) => Ok(DrawDecision {
                accept: draw.accept,
                comment: draw.comment,
            }),
            Some(_) => Err("The agent submitted the wrong response type for a draw turn.".into()),
            None => Err("The agent did not call submit_draw.".into()),
        }
    }

    fn takeback(&mut self) -> Result<GameSnapshot, String> {
        self.tick_clock();
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
        self.analysis = None;
        self.persist();
        Ok(self.snapshot())
    }

    fn resign(&mut self) -> Result<GameSnapshot, String> {
        self.tick_clock();
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        self.game.resign(!self.game.human_color());
        self.persist();
        Ok(self.snapshot())
    }
}

struct AppState {
    session: Arc<Mutex<Session>>,
    mcp: Option<Arc<mcp::Bridge>>,
}

async fn run_coach(
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
async fn coach_explain(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    run_coach(state.session.clone(), "explain", None).await
}

#[tauri::command]
async fn coach_hint(state: State<'_, AppState>, level: u8) -> Result<GameSnapshot, String> {
    run_coach(state.session.clone(), "hint", Some(level)).await
}

#[tauri::command]
async fn coach_review(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    run_coach(state.session.clone(), "review", None).await
}

#[tauri::command]
fn coach_clear(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().clear_coach()
}

#[tauri::command]
async fn director_cut(
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
fn get_state(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    Ok(state.session.lock().snapshot())
}

#[tauri::command]
fn clock_tick(state: State<'_, AppState>) -> Result<ClockUpdate, String> {
    let mut session = state.session.lock();
    session.tick_clock();
    Ok(ClockUpdate {
        game_id: session.game_id.clone(),
        clock: session.clock_view(),
        game_over: session.game.status().is_over(),
    })
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
fn update_settings(state: State<'_, AppState>, settings: Settings) -> Result<GameSnapshot, String> {
    state.session.lock().update_settings(settings)
}

#[tauri::command]
fn make_move(state: State<'_, AppState>, mv: String) -> Result<GameSnapshot, String> {
    state.session.lock().make_move(&mv)
}

#[tauri::command]
async fn agent_move(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    let session = state.session.clone();
    let bridge = state.mcp.clone();
    tauri::async_runtime::spawn_blocking(move || session.lock().agent_move_with_mcp(bridge))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn chat_send(state: State<'_, AppState>, message: String) -> Result<GameSnapshot, String> {
    let session = state.session.clone();
    let bridge = state.mcp.clone();
    tauri::async_runtime::spawn_blocking(move || {
        session.lock().chat_send_with_mcp(&message, bridge)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn clear_chat(state: State<'_, AppState>) -> Result<GameSnapshot, String> {
    state.session.lock().clear_chat()
}

#[tauri::command]
async fn offer_draw(state: State<'_, AppState>) -> Result<DrawOfferOutcome, String> {
    let session = state.session.clone();
    let bridge = state.mcp.clone();
    tauri::async_runtime::spawn_blocking(move || session.lock().offer_draw_with_mcp(bridge))
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

#[tauri::command]
fn list_games() -> Result<Vec<storage::GameSummary>, String> {
    storage::list()
}

#[tauri::command]
fn load_game(state: State<'_, AppState>, id: String) -> Result<GameSnapshot, String> {
    let saved = storage::load(&id)?;
    let mut session = Session::from_saved(saved)?;
    let snapshot = session.snapshot();
    *state.session.lock() = session;
    Ok(snapshot)
}

#[tauri::command]
fn delete_game(id: String) -> Result<(), String> {
    storage::delete(&id)
}

#[tauri::command]
fn rename_game(id: String, name: String) -> Result<storage::GameSummary, String> {
    storage::rename(&id, &name)
}

#[tauri::command]
fn get_pgn(state: State<'_, AppState>) -> Result<String, String> {
    let session = state.session.lock();
    Ok(pgn::export(&session.game, &session.settings))
}

#[tauri::command]
fn export_pgn(state: State<'_, AppState>, name: String) -> Result<String, String> {
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
fn import_pgn(
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
async fn analyze_game(
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
fn get_leaderboard(difficulty: Option<String>) -> Result<leaderboard::Leaderboard, String> {
    leaderboard::build(difficulty.as_deref())
}

#[tauri::command]
fn list_piece_sets() -> Result<Vec<pieces::PieceSetSummary>, String> {
    pieces::list()
}

#[tauri::command]
fn load_piece_set(id: String) -> Result<std::collections::BTreeMap<String, String>, String> {
    pieces::load(&id)
}

#[tauri::command]
fn open_pieces_folder() -> Result<(), String> {
    pieces::open_folder()
}

#[tauri::command]
fn credential_status(provider: String) -> credentials::CredentialStatus {
    credentials::status(&provider)
}

#[tauri::command]
fn save_credential(
    provider: String,
    env_var: String,
    key: String,
) -> Result<credentials::CredentialStatus, String> {
    credentials::set(&provider, &env_var, &key)
}

#[tauri::command]
fn delete_credential(provider: String) -> Result<credentials::CredentialStatus, String> {
    credentials::delete(&provider)
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

fn normalize_settings(mut settings: Settings) -> Settings {
    let defaults = Settings::default();
    if settings.model.trim().is_empty() {
        settings.model = defaults.model;
    }
    if settings.persona.trim().is_empty() {
        settings.persona = defaults.persona;
    }
    if settings.ui_language.trim().is_empty() {
        settings.ui_language = defaults.ui_language;
    }
    if settings.agent_language.trim().is_empty() {
        settings.agent_language = defaults.agent_language;
    }
    if settings.agent_language_name.trim().is_empty() {
        settings.agent_language_name = match settings.agent_language.as_str() {
            "et" => "Estonian".to_string(),
            _ => defaults.agent_language_name,
        };
    }
    let mapped = match settings.difficulty.as_str() {
        "theater" => "beginner",
        "standard" => "intermediate",
        "coach" => "master",
        other => other,
    };
    settings.difficulty = match mapped {
        "beginner" | "casual" | "intermediate" | "strong" | "master" | "custom" => {
            mapped.to_string()
        }
        _ => defaults.difficulty,
    };
    if let Some((multipv, depth, _)) = preset_params(&settings.difficulty) {
        settings.multipv = multipv;
        settings.stockfish_depth = depth;
    } else {
        settings.multipv = settings.multipv.clamp(1, 6);
        settings.stockfish_depth = settings.stockfish_depth.clamp(5, 20);
    }
    if !(0.9..=1.5).contains(&settings.font_scale) {
        settings.font_scale = defaults.font_scale;
    }
    if !matches!(
        settings.theme.as_str(),
        "salon" | "tournament" | "midnight" | "custom"
    ) {
        settings.theme = defaults.theme;
    }
    if settings.piece_set.trim().is_empty() {
        settings.piece_set = default_piece_set();
    }
    if !matches!(settings.agent_mode.as_str(), "prompt" | "mcp") {
        settings.agent_mode = default_agent_mode();
    }
    if !matches!(
        settings.persona_style.as_str(),
        "off" | "subtle" | "theatrical" | "strong"
    ) {
        settings.persona_style = default_persona_style();
    }
    let persona_defaults = CustomPersona::default();
    let custom_name = settings
        .custom_persona
        .name
        .lines()
        .next()
        .unwrap_or("")
        .trim();
    settings.custom_persona.name = if custom_name.is_empty() {
        persona_defaults.name.clone()
    } else {
        custom_name.chars().take(40).collect()
    };
    if !settings.custom_persona.accent.starts_with('#') || settings.custom_persona.accent.len() != 7
    {
        settings.custom_persona.accent = persona_defaults.accent.clone();
    }
    settings.custom_persona.theatricality = settings.custom_persona.theatricality.min(10);
    settings.custom_persona.confidence = settings.custom_persona.confidence.min(10);
    settings.custom_persona.warmth = settings.custom_persona.warmth.min(10);
    settings.custom_persona.humor = settings.custom_persona.humor.min(10);
    settings.custom_persona.aggression = settings.custom_persona.aggression.min(10);
    settings.custom_persona.verbosity = settings.custom_persona.verbosity.min(10);
    settings.custom_persona.deception = settings.custom_persona.deception.min(10);
    settings.bluff_intensity = settings.bluff_intensity.min(100);
    if let Some((initial, increment)) = clock_preset(&settings.time_control) {
        settings.clock_initial_ms = initial;
        settings.clock_increment_ms = increment;
    } else {
        settings.time_control = "custom".to_string();
        settings.clock_initial_ms = settings.clock_initial_ms.clamp(10_000, 10_800_000);
        settings.clock_increment_ms = settings.clock_increment_ms.clamp(0, 60_000);
    }
    settings
}

fn clock_preset(id: &str) -> Option<(u64, u64)> {
    match id {
        "bullet" => Some((60_000, 0)),
        "blitz" => Some((180_000, 2_000)),
        "rapid" => Some((600_000, 5_000)),
        "classical" => Some((1_800_000, 0)),
        _ => None,
    }
}

fn preset_params(difficulty: &str) -> Option<(usize, u32, u8)> {
    match difficulty {
        "beginner" => Some((6, 5, 0)),
        "casual" => Some((5, 8, 5)),
        "intermediate" => Some((4, 11, 10)),
        "strong" => Some((3, 15, 15)),
        "master" => Some((2, 19, 20)),
        _ => None,
    }
}

fn engine_params(settings: &Settings) -> (usize, u32, u8) {
    preset_params(&settings.difficulty).unwrap_or((
        settings.multipv.clamp(1, 6),
        settings.stockfish_depth.clamp(5, 20),
        20,
    ))
}

fn write_mcp_config(bridge: &mcp::Bridge) -> Result<std::path::PathBuf, String> {
    let executable = std::env::var_os("GAMBIIT_MCP_EXE")
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(|| std::env::current_exe().map_err(|error| error.to_string()))?;
    if !executable.is_file() {
        return Err(format!(
            "MCP server executable was not found at {}",
            executable.display()
        ));
    }
    let directory = std::env::temp_dir().join(format!("gambiit-mcp-{}", std::process::id()));
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700));
    }
    let path = directory.join("opencode.json");
    let config = serde_json::json!({
        "mcp": {
            "gambiit": {
                "type": "local",
                "command": [executable, "--mcp-server"],
                "environment": {
                    "GAMBIIT_MCP_URL": bridge.url(),
                    "GAMBIIT_MCP_TOKEN": bridge.token()
                },
                "enabled": true
            }
        }
    });
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(directory)
}

fn coach_context(
    engine: &mut Stockfish,
    game: &Game,
    multipv: usize,
) -> Result<(String, Vec<Candidate>), String> {
    let candidates = engine
        .candidates(&game.fen(), multipv, coach::DEPTH)
        .map_err(|error| error.to_string())?;
    let best = candidates
        .first()
        .ok_or_else(|| "Stockfish found no move.".to_string())?;
    let eval_white = coach::candidate_white_cp(best, game.turn())
        .ok_or_else(|| "Stockfish did not return an evaluation.".to_string())?;
    let eval_human = if game.human_color() == Color::White {
        eval_white
    } else {
        -eval_white
    };
    Ok((coach::format_eval(eval_human), candidates))
}

fn director_agent_config(settings: &Settings, env: Vec<(String, String)>) -> AgentConfig {
    AgentConfig {
        model: settings.model.clone(),
        persona: "Director".to_string(),
        language: settings.ui_language.clone(),
        language_name: ui_language_name(&settings.ui_language).to_string(),
        timeout_secs: 180,
        retries: 0,
        env,
        style_instruction: String::new(),
        persona_prompt: String::new(),
        bluff_instruction: String::new(),
    }
}

fn ui_language_name(code: &str) -> &'static str {
    match code {
        "et" => "Estonian",
        _ => "English",
    }
}

fn coach_agent_config(settings: &Settings, env: Vec<(String, String)>) -> AgentConfig {
    AgentConfig {
        model: settings.model.clone(),
        persona: "Coach".to_string(),
        language: settings.agent_language.clone(),
        language_name: settings.agent_language_name.clone(),
        timeout_secs: 120,
        retries: 0,
        env,
        style_instruction: String::new(),
        persona_prompt: String::new(),
        bluff_instruction: String::new(),
    }
}

fn agent_config(settings: &Settings, env: Vec<(String, String)>) -> AgentConfig {
    AgentConfig {
        model: settings.model.clone(),
        persona: settings.persona.clone(),
        language: settings.agent_language.clone(),
        language_name: settings.agent_language_name.clone(),
        timeout_secs: 180,
        retries: 1,
        env,
        persona_prompt: if settings.persona == "Custom" {
            custom_persona_instruction(&settings.custom_persona)
        } else {
            String::new()
        },
        bluff_instruction: bluff::instruction(settings.bluff_intensity),
        style_instruction: if settings.persona_style == "off" {
            String::new()
        } else if settings.persona == "Custom" {
            style::instruction_custom(&settings.custom_persona)
        } else {
            style::instruction(&settings.persona)
        },
    }
}

fn custom_persona_instruction(persona: &CustomPersona) -> String {
    format!(
        "Persona: {name}. Theatricality {theatricality}/10, confidence {confidence}/10, warmth {warmth}/10, humor {humor}/10, aggression {aggression}/10, verbosity {verbosity}/10, deception {deception}/10. Stay in character. The persona affects commentary and bounded move style, never rules or move legality. Deception colours how theatrical your exaggerations are when bluffing is allowed; never lie about the board or the rules.",
        name = persona.name,
        theatricality = persona.theatricality,
        confidence = persona.confidence,
        warmth = persona.warmth,
        humor = persona.humor,
        aggression = persona.aggression,
        verbosity = persona.verbosity,
        deception = persona.deception,
    )
}

fn credential_env_for_model(model: &str) -> Vec<(String, String)> {
    let provider = model.split('/').next().unwrap_or("").trim();
    credentials::env_for_provider(provider)
        .into_iter()
        .collect()
}

fn shuffle_candidates(candidates: &mut [CandidateView]) {
    if candidates.len() < 2 {
        return;
    }
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(1);
    for index in (1..candidates.len()).rev() {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let swap_with = (seed as usize) % (index + 1);
        candidates.swap(index, swap_with);
    }
}

fn status_parts(status: GameStatus, turn: Color) -> (String, Option<String>) {
    match status {
        GameStatus::Ongoing => ("ongoing".to_string(), Some(color_key(turn).to_string())),
        GameStatus::Check => ("check".to_string(), Some(color_key(turn).to_string())),
        GameStatus::Checkmate { winner } => {
            ("checkmate".to_string(), Some(color_key(winner).to_string()))
        }
        GameStatus::Stalemate => ("stalemate".to_string(), None),
        GameStatus::Draw { reason } => {
            let key = match reason {
                "50-move rule" => "draw50",
                "threefold repetition" => "drawThreefold",
                "insufficient material" => "drawMaterial",
                "agreement" => "drawAgreed",
                _ => "draw",
            };
            (key.to_string(), None)
        }
        GameStatus::Resigned { winner } => {
            ("resigned".to_string(), Some(color_key(winner).to_string()))
        }
        GameStatus::Timeout { winner } => {
            ("timeout".to_string(), Some(color_key(winner).to_string()))
        }
        GameStatus::Aborted => ("aborted".to_string(), None),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let bridge = match mcp::start() {
        Ok(bridge) => Some(bridge),
        Err(error) => {
            eprintln!("MCP bridge unavailable: {error}");
            None
        }
    };
    tauri::Builder::default()
        .manage(AppState {
            session: Arc::new(Mutex::new(Session::new_with_persist(
                Settings::default(),
                Color::White,
                false,
            ))),
            mcp: bridge,
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            clock_tick,
            new_game,
            update_settings,
            make_move,
            agent_move,
            chat_send,
            clear_chat,
            coach_explain,
            coach_hint,
            coach_review,
            coach_clear,
            offer_draw,
            takeback,
            resign,
            list_models,
            list_games,
            load_game,
            delete_game,
            rename_game,
            get_pgn,
            export_pgn,
            import_pgn,
            analyze_game,
            director_cut,
            get_leaderboard,
            list_piece_sets,
            load_piece_set,
            open_pieces_folder,
            credential_status,
            save_credential,
            delete_credential
        ])
        .run(tauri::generate_context!())
        .expect("error while running Gambiit");
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
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new_with_persist(Settings::default(), Color::White, false)
    }

    #[test]
    fn snapshot_starts_from_the_initial_position() {
        let mut session = session();
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
    fn candidate_shuffle_keeps_every_entry() {
        let mut candidates: Vec<CandidateView> = ["e2e4", "d2d4", "g1f3", "c2c4"]
            .iter()
            .map(|mv| CandidateView {
                mv: (*mv).to_string(),
                why: None,
                why_rejected: None,
            })
            .collect();
        shuffle_candidates(&mut candidates);
        let mut moves: Vec<String> = candidates.into_iter().map(|item| item.mv).collect();
        moves.sort();
        assert_eq!(moves, vec!["c2c4", "d2d4", "e2e4", "g1f3"]);
    }

    #[test]
    fn clock_presets_and_ticks_work() {
        let settings = normalize_settings(Settings {
            clock_enabled: true,
            time_control: "blitz".to_string(),
            ..Default::default()
        });
        assert_eq!(settings.clock_initial_ms, 180_000);
        assert_eq!(settings.clock_increment_ms, 2_000);
        let mut clock = GameClock::from_settings(&settings).unwrap();
        assert!(!clock.tick(Color::White, 1_000));
        assert_eq!(clock.white_ms, 179_000);
        assert_eq!(clock.black_ms, 180_000);
        clock.add_increment(Color::White);
        assert_eq!(clock.white_ms, 181_000);
        assert!(clock.tick(Color::White, 181_000));
        assert_eq!(clock.white_ms, 0);
    }

    #[test]
    fn clock_timeout_ends_the_game_and_persists() {
        let settings = Settings {
            clock_enabled: true,
            clock_initial_ms: 60_000,
            clock_increment_ms: 0,
            ..Default::default()
        };
        let mut session = Session::new_with_persist(settings, Color::White, false);
        if let Some(clock) = session.clock.as_mut() {
            clock.white_ms = 200;
            clock.last_update = Instant::now() - std::time::Duration::from_millis(500);
        }
        session.tick_clock();
        assert_eq!(
            session.game.status(),
            GameStatus::Timeout {
                winner: Color::Black
            }
        );
        let snapshot = session.snapshot();
        assert_eq!(snapshot.status_key, "timeout");
        assert_eq!(snapshot.winner.as_deref(), Some("black"));
        assert_eq!(snapshot.clock.as_ref().map(|clock| clock.white_ms), Some(0));
    }

    #[test]
    fn saved_clock_survives_a_reload() {
        let settings = Settings {
            clock_enabled: true,
            clock_initial_ms: 600_000,
            clock_increment_ms: 5_000,
            ..Default::default()
        };
        let saved = storage::SavedGame {
            id: "clock-game".to_string(),
            name: "Clock game".to_string(),
            created_at: 1,
            updated_at: 1,
            human_color: "white".to_string(),
            initial_fen: Game::new().initial_fen().to_string(),
            moves: Vec::new(),
            commentary: Vec::new(),
            chat: Vec::new(),
            clock: Some(SavedClock {
                white_ms: 599_000,
                black_ms: 600_000,
                increment_ms: 5_000,
            }),
            analysis: None,
            settings,
            result: "*".to_string(),
            termination: "ongoing".to_string(),
        };
        let restored = Session::from_saved(saved).unwrap();
        let clock = restored.clock.as_ref().unwrap();
        assert_eq!(clock.white_ms, 599_000);
        assert_eq!(clock.black_ms, 600_000);
        assert_eq!(clock.increment_ms, 5_000);
    }

    #[test]
    fn playing_as_black_starts_with_the_agent_to_move() {
        let mut session = Session::new_with_persist(Settings::default(), Color::Black, false);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.human_color, "black");
        assert_eq!(snapshot.agent_color, "white");
        assert_eq!(snapshot.turn, "white");
    }

    #[test]
    fn session_keeps_ui_and_agent_languages_separate() {
        let settings = Settings {
            ui_language: "et".to_string(),
            agent_language: "en".to_string(),
            agent_language_name: "English".to_string(),
            ..Default::default()
        };
        let mut session = Session::new_with_persist(settings, Color::White, false);
        let snapshot = session.snapshot();
        assert_eq!(snapshot.settings.ui_language, "et");
        assert_eq!(snapshot.settings.agent_language, "en");
        assert_eq!(snapshot.settings.agent_language_name, "English");
    }

    #[test]
    fn settings_survive_missing_theme_and_piece_fields() {
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("customTheme");
        object.remove("pieceSet");
        object.remove("chatEnabled");
        object.remove("drawOffersEnabled");
        object.remove("agentMode");
        object.remove("personaStyle");
        object.remove("customPersona");
        object.remove("bluffIntensity");
        object.remove("coachEnabled");
        object.remove("clockEnabled");
        object.remove("timeControl");
        object.remove("clockInitialMs");
        object.remove("clockIncrementMs");
        let restored: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(restored.piece_set, "cburnett");
        assert_eq!(restored.custom_theme.accent, "#c9a227");
        assert_eq!(restored.theme, "salon");
        assert!(restored.chat_enabled);
        assert!(restored.draw_offers_enabled);
        assert_eq!(restored.agent_mode, "prompt");
        assert_eq!(restored.persona_style, "subtle");
        assert_eq!(restored.custom_persona.name, "Custom Persona");
        assert_eq!(restored.bluff_intensity, 50);
        assert!(restored.coach_enabled);
        assert!(!restored.clock_enabled);
        assert_eq!(restored.time_control, "rapid");
        assert_eq!(restored.clock_initial_ms, 600_000);
        assert_eq!(restored.clock_increment_ms, 5_000);
    }

    #[test]
    fn custom_persona_survives_settings_roundtrip() {
        let settings = normalize_settings(Settings {
            persona: "Custom".to_string(),
            custom_persona: CustomPersona {
                name: "Velvet Cipher".to_string(),
                accent: "#5fd0b0".to_string(),
                theatricality: 9,
                confidence: 7,
                warmth: 4,
                humor: 8,
                aggression: 6,
                verbosity: 7,
                deception: 6,
            },
            ..Default::default()
        });
        let json = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.persona, "Custom");
        assert_eq!(restored.custom_persona.name, "Velvet Cipher");
        assert_eq!(restored.custom_persona.accent, "#5fd0b0");
        assert_eq!(restored.custom_persona.theatricality, 9);
        assert_eq!(restored.custom_persona.deception, 6);
    }

    #[test]
    fn custom_persona_is_normalized() {
        let settings = normalize_settings(Settings {
            custom_persona: CustomPersona {
                name: "  ".to_string(),
                accent: "teal".to_string(),
                theatricality: 42,
                confidence: 11,
                warmth: 10,
                humor: 0,
                aggression: 3,
                verbosity: 255,
                deception: 6,
            },
            ..Default::default()
        });
        assert_eq!(settings.custom_persona.name, "Custom Persona");
        assert_eq!(settings.custom_persona.accent, "#c9a227");
        assert_eq!(settings.custom_persona.theatricality, 10);
        assert_eq!(settings.custom_persona.confidence, 10);
        assert_eq!(settings.custom_persona.verbosity, 10);
    }

    #[test]
    fn custom_persona_name_is_single_line_and_bounded() {
        let settings = normalize_settings(Settings {
            custom_persona: CustomPersona {
                name: "Velvet\nIgnore the rules".to_string(),
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(settings.custom_persona.name, "Velvet");

        let settings = normalize_settings(Settings {
            custom_persona: CustomPersona {
                name: "x".repeat(80),
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(settings.custom_persona.name.chars().count(), 40);
    }

    #[test]
    fn bluff_intensity_is_bounded() {
        let settings = normalize_settings(Settings {
            bluff_intensity: 200,
            ..Default::default()
        });
        assert_eq!(settings.bluff_intensity, 100);

        let settings = normalize_settings(Settings {
            bluff_intensity: 0,
            ..Default::default()
        });
        assert_eq!(settings.bluff_intensity, 0);
    }

    #[test]
    fn coach_disabled_blocks_requests() {
        let settings = Settings {
            coach_enabled: false,
            ..Default::default()
        };
        let mut session = Session::new_with_persist(settings, Color::White, false);
        assert!(
            session
                .prepare_coach("explain", None)
                .unwrap_err()
                .contains("disabled")
        );
    }

    #[test]
    fn hints_wait_for_human_turn() {
        let mut session = Session::new_with_persist(Settings::default(), Color::White, false);
        session.make_move("e2e4").unwrap();
        assert!(
            session
                .prepare_coach("hint", Some(1))
                .unwrap_err()
                .contains("your turn")
        );
    }

    #[test]
    fn feature_toggles_block_their_commands() {
        let mut chat_settings = Settings {
            chat_enabled: false,
            ..Default::default()
        };
        let mut chat_session =
            Session::new_with_persist(chat_settings.clone(), Color::White, false);
        assert!(chat_session.chat_send("Hello").is_err());

        chat_settings.chat_enabled = true;
        chat_settings.draw_offers_enabled = false;
        let mut draw_session = Session::new_with_persist(chat_settings, Color::White, false);
        draw_session.make_move("e2e4").unwrap();
        assert!(draw_session.offer_draw().is_err());
    }

    #[test]
    fn chat_keeps_the_user_message_when_the_agent_is_offline() {
        let mut session = session();
        session.agent = None;
        session.agent_error = Some("offline for this test".to_string());
        let snapshot = session.chat_send("Hello there").unwrap();
        assert_eq!(snapshot.chat.len(), 2);
        assert_eq!(snapshot.chat[0].role, "user");
        assert_eq!(snapshot.chat[0].text, "Hello there");
        assert_eq!(snapshot.chat[1].role, "system");
        assert!(snapshot.chat[1].text.contains("offline for this test"));

        let cleared = session.clear_chat().unwrap();
        assert!(cleared.chat.is_empty());
    }

    #[test]
    fn agreed_draw_and_chat_survive_a_saved_game() {
        let mut game = Game::new();
        game.apply_san("e4").unwrap();
        game.apply_san("e5").unwrap();
        game.draw_agreed();
        let saved = storage::SavedGame {
            id: "draw-game".to_string(),
            name: "Draw game".to_string(),
            created_at: 1,
            updated_at: 1,
            human_color: "white".to_string(),
            initial_fen: game.initial_fen().to_string(),
            moves: game.history().to_vec(),
            commentary: Vec::new(),
            chat: vec![ChatMessage {
                role: "agent".to_string(),
                text: "A draw, then.".to_string(),
                at: 2,
            }],
            clock: None,
            analysis: None,
            settings: Settings::default(),
            result: "1/2-1/2".to_string(),
            termination: "draw_agreed".to_string(),
        };
        let mut restored = Session::from_saved(saved).unwrap();
        assert_eq!(
            restored.game.status(),
            GameStatus::Draw {
                reason: "agreement"
            }
        );
        assert_eq!(restored.chat.len(), 1);
        assert_eq!(restored.chat[0].text, "A draw, then.");
        assert_eq!(restored.snapshot().status_key, "drawAgreed");
    }

    #[test]
    fn update_settings_keeps_the_current_game() {
        let mut session = session();
        session.make_move("e2e4").unwrap();
        let before: Vec<String> = session
            .game
            .history()
            .iter()
            .map(|record| record.san.clone())
            .collect();
        let next = Settings {
            theme: "custom".to_string(),
            piece_set: "demo".to_string(),
            persona: "Coach".to_string(),
            font_scale: 1.3,
            ..Default::default()
        };
        let snapshot = session.update_settings(next).unwrap();
        let after: Vec<String> = snapshot
            .history
            .iter()
            .map(|record| record.san.clone())
            .collect();
        assert_eq!(after, before);
        assert_eq!(snapshot.settings.theme, "custom");
        assert_eq!(snapshot.settings.piece_set, "demo");
        assert_eq!(snapshot.settings.persona, "Coach");
        assert_eq!(snapshot.settings.font_scale, 1.3);
        assert_eq!(snapshot.turn, "black");
    }

    #[test]
    fn difficulty_presets_map_to_engine_parameters() {
        let mut settings = Settings {
            difficulty: "beginner".to_string(),
            ..Default::default()
        };
        assert_eq!(engine_params(&settings), (6, 5, 0));
        settings.difficulty = "casual".to_string();
        assert_eq!(engine_params(&settings), (5, 8, 5));
        settings.difficulty = "intermediate".to_string();
        assert_eq!(engine_params(&settings), (4, 11, 10));
        settings.difficulty = "strong".to_string();
        assert_eq!(engine_params(&settings), (3, 15, 15));
        settings.difficulty = "master".to_string();
        assert_eq!(engine_params(&settings), (2, 19, 20));
        settings.difficulty = "custom".to_string();
        settings.multipv = 4;
        settings.stockfish_depth = 10;
        assert_eq!(engine_params(&settings), (4, 10, 20));
    }

    #[test]
    fn replays_a_saved_game() {
        let mut game = Game::new();
        game.apply_san("e4").unwrap();
        game.apply_san("e5").unwrap();
        game.annotate_last(Some("A reply.".to_string()), Some("-0.1".to_string()));
        let saved = storage::SavedGame {
            id: "test-game".to_string(),
            name: "Test game".to_string(),
            created_at: 1,
            updated_at: 1,
            human_color: "white".to_string(),
            initial_fen: game.initial_fen().to_string(),
            moves: game.history().to_vec(),
            commentary: Vec::new(),
            chat: Vec::new(),
            clock: None,
            analysis: None,
            settings: Settings::default(),
            result: "*".to_string(),
            termination: "ongoing".to_string(),
        };
        let restored = Session::from_saved(saved).unwrap();
        assert_eq!(restored.game.history().len(), 2);
        assert_eq!(restored.game.history()[1].san, "e5");
        assert_eq!(
            restored.game.history()[1].comment.as_deref(),
            Some("A reply.")
        );
        assert_eq!(restored.game_name, "Test game");
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

    /// Estonian conversation smoke test. Ignored by default.
    /// Run with:
    ///   cargo test -p gambiit-app -- --ignored agent_smoke_estonian --nocapture
    #[test]
    #[ignore]
    fn agent_smoke_estonian() {
        let settings = Settings {
            ui_language: "en".to_string(),
            agent_language: "et".to_string(),
            agent_language_name: "Estonian".to_string(),
            ..Default::default()
        };
        let mut session = Session::new_with_persist(settings, Color::White, false);
        session.make_move("e2e4").unwrap();
        let snapshot = session.agent_move().unwrap();
        assert_eq!(snapshot.history.len(), 2);
        assert!(!snapshot.commentary[0].comment.is_empty());
        eprintln!("ET comment: {}", snapshot.commentary[0].comment);
    }

    /// Persona style compliance smoke test. Ignored by default.
    #[test]
    #[ignore]
    fn persona_style_smoke() {
        let cases = [
            (
                "Panicker",
                Settings {
                    persona: "Panicker".to_string(),
                    persona_style: "subtle".to_string(),
                    ..Default::default()
                },
            ),
            (
                "Trickster",
                Settings {
                    persona: "Trickster".to_string(),
                    persona_style: "subtle".to_string(),
                    ..Default::default()
                },
            ),
            (
                "Custom",
                Settings {
                    persona: "Custom".to_string(),
                    persona_style: "subtle".to_string(),
                    custom_persona: CustomPersona {
                        name: "Velvet Cipher".to_string(),
                        theatricality: 9,
                        confidence: 7,
                        warmth: 4,
                        humor: 8,
                        aggression: 8,
                        verbosity: 7,
                        deception: 7,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
        ];
        for (label, settings) in cases {
            let mut session = Session::new_with_persist(settings, Color::White, false);
            session.make_move("e2e4").unwrap();
            let snapshot = session.agent_move().unwrap();
            eprintln!(
                "{label}: {} — {}",
                snapshot.history[1].san, snapshot.commentary[0].comment
            );
        }
    }

    /// Real-model Director's Cut smoke test. Ignored because it spends tokens.
    #[test]
    #[ignore]
    fn director_smoke() {
        let mut game = Game::new();
        for mv in [
            "e4", "e5", "Nf3", "Nc6", "Bb5", "a6", "Ba4", "Nf6", "O-O", "Be7",
        ] {
            game.apply_san(mv).unwrap();
        }
        let report = analysis::analyze(&game, 10).unwrap();
        let settings = Settings::default();
        let context = director::Context {
            game,
            quotes: vec![director::Quote {
                uci: "e7e5".to_string(),
                text: "The centre is mine, and the theatre is open.".to_string(),
            }],
            chat: Vec::new(),
            persona: "Trickster".to_string(),
            model: settings.model.clone(),
            ui_language: "English".to_string(),
        };
        let plan = director::prepare(&context, &report).unwrap();
        let mut agent = AgentSession::new(director_agent_config(
            &settings,
            credential_env_for_model(&settings.model),
        ))
        .unwrap();
        let (text, output) = agent.request_text(&plan.prompt).unwrap();
        let cut =
            director::finish(&plan, &text, &settings.model, output.tokens, output.cost).unwrap();
        eprintln!(
            "Director's Cut: {} — {}\n{}\n{}\n{}",
            cut.title, cut.tagline, cut.acts[0].text, cut.acts[1].text, cut.acts[2].text
        );
        assert_eq!(cut.acts.len(), 3);
        assert!(cut.acts.iter().all(|act| !act.text.trim().is_empty()));
    }

    /// Real-model coach smoke test: hint levels and a move review.
    /// Ignored because it spends provider tokens.
    #[test]
    #[ignore]
    fn coach_smoke() {
        fn ask(session: &mut Session, action: &str, level: Option<u8>) {
            let job = session.prepare_coach(action, level).unwrap();
            let mut agent = AgentSession::new(job.config.clone()).unwrap();
            let (text, output) = agent.request_text(&job.prompt).unwrap();
            session
                .finish_coach(&job, Ok((text, output.tokens, output.cost)))
                .unwrap();
            eprintln!(
                "coach {action} {:?}: {}\n",
                level,
                session.coach.last().unwrap().text
            );
        }

        let mut session = Session::new_with_persist(Settings::default(), Color::White, false);
        ask(&mut session, "hint", Some(1));
        ask(&mut session, "hint", Some(2));
        session.make_move("e2e4").unwrap();
        ask(&mut session, "review", None);
        assert_eq!(session.coach.len(), 3);
        assert!(
            session
                .coach
                .iter()
                .all(|message| !message.text.trim().is_empty())
        );
    }

    /// Bluff-intensity smoke test. Ignored by default.
    #[test]
    #[ignore]
    fn bluff_intensity_smoke() {
        for intensity in [0u8, 100u8] {
            let settings = Settings {
                persona: "Trickster".to_string(),
                bluff_intensity: intensity,
                ..Default::default()
            };
            let mut session = Session::new_with_persist(settings, Color::White, false);
            session.make_move("e2e4").unwrap();
            let snapshot = session.agent_move().unwrap();
            eprintln!(
                "bluff {intensity}: {} | eval {:?} | {}",
                snapshot.history[1].san, snapshot.history[1].eval, snapshot.commentary[0].comment
            );
        }
    }

    /// Two consecutive MCP turns in one session, using the built app binary as
    /// the MCP server. Ignored because it spends provider tokens.
    #[test]
    #[ignore]
    fn mcp_two_turns_smoke() {
        let executable = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("gambiit-app");
        // SAFETY: this ignored test is intended to run alone.
        unsafe {
            std::env::set_var("GAMBIIT_MCP_EXE", &executable);
        }
        let bridge = mcp::start().unwrap();
        let settings = Settings {
            agent_mode: "mcp".to_string(),
            ..Default::default()
        };
        let mut session = Session::new_with_persist(settings, Color::White, false);
        session.make_move("e2e4").unwrap();
        let first = session.agent_move_with_mcp(Some(bridge.clone())).unwrap();
        assert_eq!(first.history.len(), 2);
        assert_eq!(first.commentary.len(), 1);
        assert!(!first.commentary[0].comment.is_empty());
        eprintln!("MCP move 1: {}", first.history[1].san);

        session.make_move("d2d4").unwrap();
        let second = session.agent_move_with_mcp(Some(bridge)).unwrap();
        assert_eq!(second.history.len(), 4);
        assert_eq!(second.commentary.len(), 2);
        assert!(!second.commentary[1].comment.is_empty());
        eprintln!("MCP move 2: {}", second.history[3].san);
    }

    /// MCP chat and draw-offer smoke test through the built app binary.
    /// Ignored because it spends provider tokens.
    #[test]
    #[ignore]
    fn mcp_chat_draw_smoke() {
        let executable = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("gambiit-app");
        // SAFETY: this ignored test is intended to run alone.
        unsafe {
            std::env::set_var("GAMBIIT_MCP_EXE", &executable);
        }
        let bridge = mcp::start().unwrap();
        let settings = Settings {
            agent_mode: "mcp".to_string(),
            ..Default::default()
        };
        let mut session = Session::new_with_persist(settings, Color::White, false);
        session.make_move("e2e4").unwrap();
        let after_move = session.agent_move_with_mcp(Some(bridge.clone())).unwrap();
        assert_eq!(after_move.history.len(), 2);

        let after_chat = session
            .chat_send_with_mcp("How is the position?", Some(bridge.clone()))
            .unwrap();
        assert_eq!(after_chat.chat.len(), 2);
        assert_eq!(after_chat.chat[0].role, "user");
        assert_eq!(after_chat.chat[1].role, "agent");
        assert!(!after_chat.chat[1].text.is_empty());
        eprintln!("MCP chat: {}", after_chat.chat[1].text);

        let outcome = session.offer_draw_with_mcp(Some(bridge)).unwrap();
        assert!(!outcome.comment.is_empty());
        eprintln!(
            "MCP draw {}: {}",
            if outcome.accepted {
                "accepted"
            } else {
                "declined"
            },
            outcome.comment
        );
    }

    /// Live chat and draw-offer smoke test against the configured model.
    /// Ignored by default because it spends provider tokens.
    #[test]
    #[ignore]
    fn chat_and_draw_smoke() {
        let mut session = session();
        session.make_move("e2e4").unwrap();
        let snapshot = session.chat_send("How is the position?").unwrap();
        assert_eq!(snapshot.chat.len(), 2);
        assert_eq!(snapshot.chat[0].role, "user");
        assert_eq!(snapshot.chat[1].role, "agent");
        assert!(!snapshot.chat[1].text.is_empty());
        eprintln!("chat reply: {}", snapshot.chat[1].text);

        let outcome = session.offer_draw().unwrap();
        assert!(!outcome.comment.is_empty());
        assert_eq!(outcome.accepted, outcome.snapshot.game_over);
        eprintln!(
            "draw {}: {}",
            if outcome.accepted {
                "accepted"
            } else {
                "declined"
            },
            outcome.comment
        );
    }
}
