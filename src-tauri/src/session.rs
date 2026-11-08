//! Game session state, clocks, persisted snapshots and agent turns.

use gambiit_agent::{
    AgentCancel, AgentConfig, AgentSession, AgentTurn, CANCELLED_MESSAGE, DrawDecision,
};
use gambiit_core::{Game, GameStatus, MoveInfo, MoveRecord, color_key};
use gambiit_engine::{Candidate, Stockfish};
use serde::{Deserialize, Serialize};
use shakmaty::{Color, Role, Square};
use std::{
    hash::{Hash, Hasher},
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::settings::{
    Settings, agent_config, coach_agent_config, credential_env_for_model, engine_params,
    normalize_settings,
};
use crate::{analysis, coach, mcp, pgn, storage, style};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PieceView {
    pub(crate) color: String,
    pub(crate) role: String,
    pub(crate) glyph: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SquareView {
    pub(crate) square: String,
    pub(crate) piece: Option<PieceView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CandidateView {
    #[serde(rename = "move")]
    pub(crate) mv: String,
    pub(crate) why: Option<String>,
    pub(crate) why_rejected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommentaryCard {
    pub(crate) san: String,
    pub(crate) uci: String,
    pub(crate) comment: String,
    pub(crate) eval: Option<String>,
    pub(crate) candidates: Vec<CandidateView>,
    pub(crate) plan: Option<String>,
    pub(crate) tokens: Option<u64>,
    pub(crate) cost: Option<f64>,
    pub(crate) elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) text: String,
    pub(crate) at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CoachMessage {
    pub(crate) action: String,
    pub(crate) level: Option<u8>,
    pub(crate) text: String,
    pub(crate) model: String,
    pub(crate) tokens: Option<u64>,
    pub(crate) cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClockView {
    pub(crate) white_ms: u64,
    pub(crate) black_ms: u64,
    pub(crate) increment_ms: u64,
    pub(crate) running: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedClock {
    pub white_ms: u64,
    pub black_ms: u64,
    pub increment_ms: u64,
}

pub(crate) struct GameClock {
    pub(crate) white_ms: u64,
    pub(crate) black_ms: u64,
    pub(crate) increment_ms: u64,
    pub(crate) last_update: Instant,
}

impl GameClock {
    pub(crate) fn from_settings(settings: &Settings) -> Option<Self> {
        settings.clock_enabled.then(|| Self {
            white_ms: settings.clock_initial_ms,
            black_ms: settings.clock_initial_ms,
            increment_ms: settings.clock_increment_ms,
            last_update: Instant::now(),
        })
    }

    pub(crate) fn from_saved(saved: SavedClock) -> Self {
        Self {
            white_ms: saved.white_ms,
            black_ms: saved.black_ms,
            increment_ms: saved.increment_ms,
            last_update: Instant::now(),
        }
    }

    pub(crate) fn view(&self, running: Option<Color>) -> ClockView {
        ClockView {
            white_ms: self.white_ms,
            black_ms: self.black_ms,
            increment_ms: self.increment_ms,
            running: running.map(|color| color_key(color).to_string()),
        }
    }

    pub(crate) fn tick(&mut self, turn: Color, elapsed_ms: u64) -> bool {
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

    pub(crate) fn add_increment(&mut self, color: Color) {
        let remaining = match color {
            Color::White => &mut self.white_ms,
            Color::Black => &mut self.black_ms,
        };
        *remaining = remaining.saturating_add(self.increment_ms);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClockUpdate {
    pub(crate) game_id: String,
    pub(crate) clock: Option<ClockView>,
    pub(crate) game_over: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DrawOfferOutcome {
    pub(crate) accepted: bool,
    pub(crate) comment: String,
    pub(crate) snapshot: GameSnapshot,
}

#[derive(Clone, Debug)]
pub(crate) struct CoachJob {
    pub(crate) game_id: String,
    pub(crate) fen: String,
    pub(crate) history_len: usize,
    pub(crate) action: String,
    pub(crate) level: Option<u8>,
    pub(crate) prompt: String,
    pub(crate) config: AgentConfig,
    pub(crate) cancel: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameSnapshot {
    pub(crate) game_id: String,
    pub(crate) game_name: String,
    pub(crate) fen: String,
    pub(crate) turn: String,
    pub(crate) human_color: String,
    pub(crate) agent_color: String,
    pub(crate) status_text: String,
    pub(crate) status_key: String,
    pub(crate) status_side: Option<String>,
    pub(crate) game_over: bool,
    pub(crate) winner: Option<String>,
    pub(crate) check: bool,
    pub(crate) check_square: Option<String>,
    pub(crate) legal_moves: Vec<MoveInfo>,
    pub(crate) history: Vec<MoveRecord>,
    pub(crate) last_move: Option<MoveRecord>,
    pub(crate) board: Vec<SquareView>,
    pub(crate) commentary: Vec<CommentaryCard>,
    pub(crate) chat: Vec<ChatMessage>,
    pub(crate) coach: Vec<CoachMessage>,
    pub(crate) coach_thinking: bool,
    pub(crate) clock: Option<ClockView>,
    pub(crate) settings: Settings,
    pub(crate) agent_error: Option<String>,
    pub(crate) thinking: bool,
}

pub(crate) struct Session {
    pub(crate) game: Game,
    pub(crate) settings: Settings,
    pub(crate) agent: Option<AgentSession>,
    pub(crate) agent_error: Option<String>,
    pub(crate) stockfish: Option<Stockfish>,
    pub(crate) commentary: Vec<CommentaryCard>,
    pub(crate) chat: Vec<ChatMessage>,
    pub(crate) coach: Vec<CoachMessage>,
    pub(crate) coach_thinking: bool,
    pub(crate) clock: Option<GameClock>,
    pub(crate) analysis: Option<analysis::AnalysisSummary>,
    pub(crate) thinking: bool,
    pub(crate) game_id: String,
    pub(crate) game_name: String,
    pub(crate) created_at: u64,
    pub(crate) persist_enabled: bool,
    cancel: AgentCancel,
}

impl Session {
    pub(crate) fn new(settings: Settings, human_color: Color) -> Self {
        Self::new_with_persist(settings, human_color, true)
    }

    pub(crate) fn new_with_persist(
        settings: Settings,
        human_color: Color,
        persist_enabled: bool,
    ) -> Self {
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

    pub(crate) fn from_saved(saved: storage::SavedGame) -> Result<Self, String> {
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

    pub(crate) fn from_import(
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
    pub(crate) fn assemble(
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

        Self {
            game,
            settings,
            agent,
            agent_error,
            stockfish: None,
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
            cancel: AgentCancel::default(),
        }
    }

    /// Share the cancellation handle with the Tauri state so Cancel does not
    /// need to wait for the session lock held during an agent request.
    pub(crate) fn use_cancel(&mut self, cancel: AgentCancel) {
        self.cancel = cancel;
    }

    fn begin_agent_request(&mut self) -> Arc<AtomicBool> {
        let token = self.cancel.begin();
        if let Some(agent) = self.agent.as_mut() {
            agent.set_cancel(Some(token.clone()));
        }
        token
    }

    /// Start Stockfish on first use instead of on every session creation.
    pub(crate) fn engine(&mut self) -> Result<&mut Stockfish, String> {
        if self.stockfish.is_none() {
            let path = Stockfish::discover().ok_or_else(|| {
                "Stockfish is not installed. Install it to enable the coach and analysis."
                    .to_string()
            })?;
            let engine = Stockfish::new(&path)
                .map_err(|error| format!("Stockfish could not start: {error}"))?;
            self.stockfish = Some(engine);
        }
        Ok(self
            .stockfish
            .as_mut()
            .expect("Stockfish was initialized above"))
    }

    pub(crate) fn saved(&self) -> storage::SavedGame {
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

    pub(crate) fn persist(&self) {
        if !self.persist_enabled {
            return;
        }
        if let Err(error) = storage::save(&self.saved()) {
            eprintln!("Could not save game: {error}");
        }
    }

    pub(crate) fn update_settings(&mut self, next: Settings) -> Result<GameSnapshot, String> {
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

    pub(crate) fn prepare_coach(
        &mut self,
        action: &str,
        level: Option<u8>,
    ) -> Result<CoachJob, String> {
        if !self.settings.coach_enabled {
            return Err("Coach is disabled in settings.".to_string());
        }
        if self.coach_thinking {
            return Err("The coach is already thinking.".to_string());
        }
        if self.game.status().is_over() {
            return Err("The game is over.".to_string());
        }

        let cancel = self.begin_agent_request();
        let game = self.game.clone();
        let human_color = game.human_color();
        let language = self.settings.agent_language_name.clone();

        let (prompt, level) = match action {
            "explain" => {
                let (eval_text, _) = {
                    let engine = self
                        .engine()
                        .map_err(|error| format!("The coach needs Stockfish. {error}"))?;
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
                    let engine = self
                        .engine()
                        .map_err(|error| format!("The coach needs Stockfish. {error}"))?;
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
                    let engine = self
                        .engine()
                        .map_err(|error| format!("The coach needs Stockfish. {error}"))?;
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
            cancel,
        })
    }

    pub(crate) fn finish_coach(
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

    pub(crate) fn clear_coach(&mut self) -> Result<GameSnapshot, String> {
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
    pub(crate) fn chat_send(&mut self, message: &str) -> Result<GameSnapshot, String> {
        self.chat_send_with_mcp(message, None)
    }

    pub(crate) fn chat_send_with_mcp(
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

        self.begin_agent_request();
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

        if result
            .as_ref()
            .err()
            .is_some_and(|error| error == CANCELLED_MESSAGE)
        {
            return Err(CANCELLED_MESSAGE.to_string());
        }
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

    pub(crate) fn clear_chat(&mut self) -> Result<GameSnapshot, String> {
        self.chat.clear();
        self.persist();
        Ok(self.snapshot())
    }

    #[cfg(test)]
    pub(crate) fn offer_draw(&mut self) -> Result<DrawOfferOutcome, String> {
        self.offer_draw_with_mcp(None)
    }

    pub(crate) fn offer_draw_with_mcp(
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

        self.begin_agent_request();
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

    pub(crate) fn tick_clock(&mut self) {
        if self.clock.is_none() {
            return;
        }
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

    pub(crate) fn add_increment(&mut self, color: Color) {
        if let Some(clock) = self.clock.as_mut() {
            clock.add_increment(color);
            clock.last_update = Instant::now();
        }
    }

    pub(crate) fn clock_view(&self) -> Option<ClockView> {
        self.clock_view_for(self.game.status().is_over())
    }

    pub(crate) fn clock_view_for(&self, game_over: bool) -> Option<ClockView> {
        let clock = self.clock.as_ref()?;
        let running = if game_over {
            None
        } else {
            Some(self.game.turn())
        };
        Some(clock.view(running))
    }

    /// Cheap change detector for the read-only API.
    ///
    /// The SSE poller must not build and serialize a full snapshot when nothing
    /// has changed. Every field that reaches `GameSnapshot` is represented
    /// here, either directly or through a derived value.
    pub(crate) fn state_fingerprint(&mut self) -> u64 {
        self.tick_clock();
        let status = self.game.status();
        let clock = self.clock_view_for(status.is_over());
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.game_id.hash(&mut hasher);
        self.game_name.hash(&mut hasher);
        self.game.fen().hash(&mut hasher);
        color_key(self.game.human_color()).hash(&mut hasher);
        let (status_key, status_side) = status_parts(status, self.game.turn());
        status_key.hash(&mut hasher);
        status_side.hash(&mut hasher);
        self.game.history().len().hash(&mut hasher);
        hash_json(&self.game.last_move(), &mut hasher);
        self.commentary.len().hash(&mut hasher);
        hash_json(&self.commentary.last(), &mut hasher);
        self.chat.len().hash(&mut hasher);
        hash_json(&self.chat.last(), &mut hasher);
        self.coach.len().hash(&mut hasher);
        hash_json(&self.coach.last(), &mut hasher);
        self.coach_thinking.hash(&mut hasher);
        self.thinking.hash(&mut hasher);
        self.agent_error.hash(&mut hasher);
        match clock {
            Some(clock) => {
                clock_bucket(clock.white_ms).hash(&mut hasher);
                clock_bucket(clock.black_ms).hash(&mut hasher);
                clock.increment_ms.hash(&mut hasher);
                clock.running.hash(&mut hasher);
            }
            None => 0u8.hash(&mut hasher),
        }
        hash_json(&self.settings, &mut hasher);
        hasher.finish()
    }

    pub(crate) fn snapshot(&mut self) -> GameSnapshot {
        self.tick_clock();
        let turn = self.game.turn();
        let status = self.game.status();
        let check = self.game.is_check();
        let mut board = Vec::with_capacity(64);
        let mut check_square = None;

        for rank in (0..8).rev() {
            for file in 0..8 {
                let square = Square::new(rank * 8 + file);
                let piece = self.game.piece_at(square);
                if check
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

        let (status_key, status_side) = status_parts(status, turn);
        GameSnapshot {
            game_id: self.game_id.clone(),
            game_name: self.game_name.clone(),
            fen: self.game.fen(),
            turn: color_key(turn).to_string(),
            human_color: color_key(self.game.human_color()).to_string(),
            agent_color: color_key(!self.game.human_color()).to_string(),
            status_text: self.game.status_text_for(status),
            status_key,
            status_side,
            game_over: status.is_over(),
            winner: status.winner().map(|color| color_key(color).to_string()),
            check,
            check_square,
            legal_moves: self.game.legal_moves(),
            history: self.game.history().to_vec(),
            last_move: self.game.last_move().cloned(),
            board,
            commentary: self.commentary.clone(),
            chat: self.chat.clone(),
            coach: self.coach.clone(),
            coach_thinking: self.coach_thinking,
            clock: self.clock_view_for(status.is_over()),
            settings: self.settings.clone(),
            agent_error: self.agent_error.clone(),
            thinking: self.thinking,
        }
    }

    pub(crate) fn make_move(&mut self, mv: &str) -> Result<GameSnapshot, String> {
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
    pub(crate) fn agent_move(&mut self) -> Result<GameSnapshot, String> {
        self.agent_move_with_mcp(None)
    }

    pub(crate) fn agent_move_with_mcp(
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

    pub(crate) fn agent_move_inner(
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

        let cancel = self.begin_agent_request();
        let (multipv, depth, skill) = engine_params(&self.settings);
        let fen = self.game.fen();
        let mut candidates = match self.engine() {
            Ok(engine) => {
                if let Err(error) = engine.set_skill(skill) {
                    eprintln!("Stockfish skill warning: {error}");
                }
                match engine.candidates(&fen, multipv, depth) {
                    Ok(candidates) => candidates,
                    Err(error) => {
                        eprintln!("Stockfish warning: {error}");
                        Vec::new()
                    }
                }
            }
            Err(error) => {
                eprintln!("Stockfish unavailable: {error}");
                Vec::new()
            }
        };

        if cancel.load(Ordering::Relaxed) {
            return Err(CANCELLED_MESSAGE.to_string());
        }
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
                if error != CANCELLED_MESSAGE {
                    self.agent_error = Some(error.clone());
                }
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

    pub(crate) fn analysis_value(&self) -> Option<serde_json::Value> {
        self.analysis
            .as_ref()
            .and_then(|summary| serde_json::to_value(summary).ok())
    }

    pub(crate) fn mcp_candidates(&mut self) -> Vec<Candidate> {
        let (multipv, depth, skill) = engine_params(&self.settings);
        let fen = self.game.fen();
        let Some(engine) = self.engine().ok() else {
            return Vec::new();
        };
        if let Err(error) = engine.set_skill(skill) {
            eprintln!("Stockfish skill warning: {error}");
        }
        engine.candidates(&fen, multipv, depth).unwrap_or_default()
    }

    pub(crate) fn begin_mcp_turn(
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

    pub(crate) fn request_mcp_turn(
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

    pub(crate) fn request_mcp_chat(
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

    pub(crate) fn request_mcp_draw(
        &mut self,
        bridge: &Arc<mcp::Bridge>,
    ) -> Result<DrawDecision, String> {
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

    pub(crate) fn takeback(&mut self) -> Result<GameSnapshot, String> {
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

    pub(crate) fn resign(&mut self) -> Result<GameSnapshot, String> {
        self.tick_clock();
        if self.game.status().is_over() {
            return Err("The game is over.".into());
        }
        self.game.resign(!self.game.human_color());
        self.persist();
        Ok(self.snapshot())
    }
}

pub(crate) fn parse_color(value: &str) -> Option<Color> {
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

pub(crate) fn shuffle_candidates(candidates: &mut [CandidateView]) {
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

fn hash_json<T: Serialize + ?Sized>(value: &T, hasher: &mut impl Hasher) {
    match serde_json::to_vec(value) {
        Ok(bytes) => bytes.hash(hasher),
        Err(_) => 0u8.hash(hasher),
    }
}

/// Quantize clock values to display precision so a running clock does not
/// force a full snapshot serialization on every poll.
fn clock_bucket(ms: u64) -> u64 {
    if ms < 20_000 { ms / 100 } else { ms / 1_000 }
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

pub(crate) fn write_mcp_config(bridge: &mcp::Bridge) -> Result<std::path::PathBuf, String> {
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
    let mut config = gambiit_agent::restricted_agent_config();
    if let Some(object) = config.as_object_mut() {
        object.insert(
            "mcp".to_string(),
            serde_json::json!({
                "gambiit": {
                    "type": "local",
                    "command": [executable, "--mcp-server"],
                    "environment": {
                        "GAMBIIT_MCP_URL": bridge.url(),
                        "GAMBIIT_MCP_TOKEN": bridge.token()
                    },
                    "enabled": true
                }
            }),
        );
    }
    let contents = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    let temporary = directory.join(".opencode.json.tmp");
    {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(contents.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().ok();
    }
    std::fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
    Ok(directory)
}
