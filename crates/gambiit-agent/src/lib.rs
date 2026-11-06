//! OpenCode prompt transport and agent response contract.
//!
//! The agent is allowed to be theatrical. The game is not: every move returned
//! here is validated against `gambiit-core` before it reaches the board.

use gambiit_core::{Game, MoveInfo, color_name};
use gambiit_engine::Candidate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("opencode was not found on PATH")]
    OpenCodeNotFound,
    #[error("failed to run opencode: {0}")]
    Io(#[from] std::io::Error),
    #[error("opencode timed out after {0} seconds")]
    Timeout(u64),
    #[error("opencode failed: {0}")]
    OpenCodeFailed(String),
    #[error("could not parse the agent response: {0}")]
    Parse(String),
    #[error("agent move was rejected: {0}")]
    InvalidMove(String),
    #[error("agent returned no usable text")]
    EmptyResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCandidate {
    #[serde(rename = "move")]
    pub mv: String,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub why_rejected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTurn {
    #[serde(rename = "move")]
    pub mv: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub eval: Option<String>,
    #[serde(default)]
    pub candidates: Vec<AgentCandidate>,
    #[serde(default)]
    pub plan: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DrawDecision {
    pub accept: bool,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: String,
    pub persona: String,
    pub language: String,
    pub language_name: String,
    pub timeout_secs: u64,
    pub retries: usize,
    /// Extra environment variables for the child process, e.g. provider API keys.
    pub env: Vec<(String, String)>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "opencode-go/deepseek-v4.1-flash".to_string(),
            persona: "Theater Master".to_string(),
            language: "en".to_string(),
            language_name: "English".to_string(),
            timeout_secs: 180,
            retries: 1,
            env: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenCodeOutput {
    pub text: String,
    pub session_id: Option<String>,
    pub cost: Option<f64>,
    pub tokens: Option<u64>,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct OpenCodeClient {
    binary: PathBuf,
    timeout_secs: u64,
}

impl OpenCodeClient {
    pub fn new(timeout_secs: u64) -> Result<Self, AgentError> {
        let binary = discover_opencode_binary().ok_or(AgentError::OpenCodeNotFound)?;
        Ok(Self {
            binary,
            timeout_secs,
        })
    }

    pub fn binary(&self) -> &PathBuf {
        &self.binary
    }

    /// Run one prompt through the OpenCode CLI and collect the text events.
    ///
    /// `--pure` keeps the game independent of the user's OpenCode plugins.
    /// `--format json` emits NDJSON; we parse the assistant text parts and the
    /// session id, ignoring unrelated events.
    pub fn run(
        &self,
        config: &AgentConfig,
        prompt: &str,
        session: Option<&str>,
    ) -> Result<OpenCodeOutput, AgentError> {
        self.run_with_env(config, prompt, session, &[], None)
    }

    pub fn run_with_env(
        &self,
        config: &AgentConfig,
        prompt: &str,
        session: Option<&str>,
        extra_env: &[(String, String)],
        working_dir: Option<&Path>,
    ) -> Result<OpenCodeOutput, AgentError> {
        let mut command = Command::new(&self.binary);
        if let Some(directory) = working_dir {
            command.current_dir(directory);
            command.env("PWD", directory);
        }
        for (name, value) in &config.env {
            command.env(name, value);
        }
        for (name, value) in extra_env {
            command.env(name, value);
        }
        // The desktop CLI talks to its background service by default. A custom
        // provider key needs a private server so the child environment is used.
        if !config.env.is_empty()
            && self.binary.file_name().and_then(|name| name.to_str()) == Some("opencode-cli")
        {
            command.arg("--standalone");
        }
        command
            .arg("run")
            .arg("--format")
            .arg("json")
            .arg("-m")
            .arg(&config.model)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(session) = session {
            command.arg("-s").arg(session);
        }
        command.arg(prompt);

        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AgentError::OpenCodeFailed("missing stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AgentError::OpenCodeFailed("missing stderr".into()))?;

        let stdout_reader = thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut stdout = stdout;
            let _ = stdout.read_to_end(&mut buffer);
            buffer
        });
        let stderr_reader = thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut stderr = stderr;
            let _ = stderr.read_to_end(&mut buffer);
            buffer
        });

        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() > Duration::from_secs(self.timeout_secs) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(AgentError::Timeout(self.timeout_secs));
            }
            thread::sleep(Duration::from_millis(150));
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| AgentError::OpenCodeFailed("stdout reader panicked".into()))?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| AgentError::OpenCodeFailed("stderr reader panicked".into()))?;
        let stdout = String::from_utf8_lossy(&stdout).to_string();
        let stderr = String::from_utf8_lossy(&stderr).to_string();

        let parsed = parse_opencode_events(&stdout);
        if !status.success() && parsed.text.trim().is_empty() {
            let detail = if stderr.trim().is_empty() {
                format!("exit status {status}")
            } else {
                stderr.trim().to_string()
            };
            return Err(AgentError::OpenCodeFailed(detail));
        }

        if parsed.text.trim().is_empty() {
            return Err(AgentError::EmptyResponse);
        }

        Ok(OpenCodeOutput {
            text: parsed.text,
            session_id: parsed.session_id,
            cost: parsed.cost,
            tokens: parsed.tokens,
            stderr,
        })
    }
}

#[derive(Debug, Default)]
struct ParsedEvents {
    text: String,
    session_id: Option<String>,
    cost: Option<f64>,
    tokens: Option<u64>,
}

fn parse_opencode_events(stdout: &str) -> ParsedEvents {
    let mut parsed = ParsedEvents::default();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if parsed.session_id.is_none() {
            parsed.session_id = value
                .get("sessionID")
                .and_then(Value::as_str)
                .map(String::from);
        }

        if value.get("type").and_then(Value::as_str) == Some("text")
            && let Some(text) = value
                .get("part")
                .and_then(|part| part.get("text"))
                .and_then(Value::as_str)
        {
            parsed.text.push_str(text);
        }

        if value.get("type").and_then(Value::as_str) == Some("step_finish")
            && let Some(part) = value.get("part")
        {
            if let Some(cost) = part.get("cost").and_then(Value::as_f64) {
                parsed.cost = Some(cost);
            }
            if let Some(tokens) = part.get("tokens") {
                let total = tokens.get("total").and_then(Value::as_u64).or_else(|| {
                    let input = tokens.get("input").and_then(Value::as_u64).unwrap_or(0);
                    let output = tokens.get("output").and_then(Value::as_u64).unwrap_or(0);
                    let reasoning = tokens.get("reasoning").and_then(Value::as_u64).unwrap_or(0);
                    let sum = input + output + reasoning;
                    (sum > 0).then_some(sum)
                });
                if total.is_some() {
                    parsed.tokens = total;
                }
            }
        }
    }
    parsed
}

pub struct AgentSession {
    client: OpenCodeClient,
    config: AgentConfig,
    session_id: Option<String>,
    last_raw: Option<String>,
}

impl AgentSession {
    pub fn new(config: AgentConfig) -> Result<Self, AgentError> {
        let client = OpenCodeClient::new(config.timeout_secs)?;
        Ok(Self {
            client,
            config,
            session_id: None,
            last_raw: None,
        })
    }

    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Update the prompt configuration, keeping the current conversation
    /// unless the model itself changes.
    pub fn update_config(&mut self, config: AgentConfig) {
        let model_changed = self.config.model != config.model;
        self.config = config;
        if model_changed {
            self.session_id = None;
            self.last_raw = None;
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn set_session_id(&mut self, session_id: Option<String>) {
        self.session_id = session_id;
    }

    pub fn last_raw(&self) -> Option<&str> {
        self.last_raw.as_deref()
    }

    /// Request a move, validating it with the supplied closure. Invalid or
    /// malformed answers are retried with the error in the prompt.
    pub fn request_move<F>(
        &mut self,
        game: &Game,
        candidates: &[Candidate],
        validate: F,
    ) -> Result<(AgentTurn, OpenCodeOutput), AgentError>
    where
        F: Fn(&AgentTurn) -> Result<(), String>,
    {
        let mut last_error = None;
        let mut fallback = None;
        for attempt in 0..=self.config.retries {
            let prompt = if attempt == 0 {
                build_move_prompt(game, &self.config, candidates)
            } else {
                build_retry_prompt(
                    game,
                    &self.config,
                    candidates,
                    self.last_raw.as_deref().unwrap_or(""),
                    last_error.as_deref().unwrap_or("invalid response"),
                )
            };

            let output = match self
                .client
                .run(&self.config, &prompt, self.session_id.as_deref())
            {
                Ok(output) => output,
                Err(error) => {
                    if let Some((turn, output)) = fallback.take() {
                        return Ok((turn, output));
                    }
                    return Err(error);
                }
            };
            if output.session_id.is_some() {
                self.session_id = output.session_id.clone();
            }
            self.last_raw = Some(output.text.clone());

            match parse_agent_turn(&output.text) {
                Ok(mut turn) => match validate(&turn) {
                    Ok(()) => {
                        if turn.comment.trim().is_empty() {
                            last_error = Some(
                                "the JSON response must include a non-empty `comment` field for the Theater panel"
                                    .to_string(),
                            );
                            if fallback.is_none() {
                                turn.comment = fallback_comment(game, &self.config, &turn.mv);
                                fallback = Some((turn, output));
                            }
                        } else {
                            return Ok((turn, output));
                        }
                    }
                    Err(error) => {
                        last_error = Some(error);
                    }
                },
                Err(error) => {
                    last_error = Some(error.to_string());
                }
            }
        }

        if let Some((turn, output)) = fallback {
            return Ok((turn, output));
        }
        Err(AgentError::InvalidMove(last_error.unwrap_or_else(|| {
            "agent did not return a valid move".into()
        })))
    }

    /// Ask the agent for a plain-text chat reply. Chat never changes the board.
    pub fn request_chat(
        &mut self,
        game: &Game,
        history: &[(String, String)],
        message: &str,
    ) -> Result<(String, OpenCodeOutput), AgentError> {
        let prompt = build_chat_prompt(game, &self.config, history, message);
        let output = self
            .client
            .run(&self.config, &prompt, self.session_id.as_deref())?;
        if output.session_id.is_some() {
            self.session_id = output.session_id.clone();
        }
        let reply: String = output.text.trim().chars().take(4_000).collect();
        if reply.is_empty() {
            return Err(AgentError::EmptyResponse);
        }
        Ok((reply, output))
    }

    /// Run one prompt with a temporary OpenCode config that registers the
    /// Gambiit MCP server. The model is expected to call `submit_move`.
    pub fn request_mcp_move(
        &mut self,
        prompt: &str,
        working_dir: &Path,
    ) -> Result<OpenCodeOutput, AgentError> {
        let output = self.client.run_with_env(
            &self.config,
            prompt,
            self.session_id.as_deref(),
            &[],
            Some(working_dir),
        )?;
        if output.session_id.is_some() {
            self.session_id = output.session_id.clone();
        }
        self.last_raw = Some(output.text.clone());
        Ok(output)
    }

    /// Ask the agent whether to accept a draw offer. The response is a small
    /// JSON object; the caller decides how to apply it to the game.
    pub fn request_draw_decision(
        &mut self,
        game: &Game,
    ) -> Result<(DrawDecision, OpenCodeOutput), AgentError> {
        let mut last_error = None;
        for attempt in 0..=self.config.retries {
            let prompt = if attempt == 0 {
                build_draw_prompt(game, &self.config)
            } else {
                build_draw_retry_prompt(
                    game,
                    &self.config,
                    last_error.as_deref().unwrap_or("invalid response"),
                )
            };
            let output = self
                .client
                .run(&self.config, &prompt, self.session_id.as_deref())?;
            if output.session_id.is_some() {
                self.session_id = output.session_id.clone();
            }
            match parse_draw_decision(&output.text) {
                Ok(decision) => return Ok((decision, output)),
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        Err(AgentError::Parse(last_error.unwrap_or_else(|| {
            "agent did not return a draw decision".into()
        })))
    }
}

pub fn build_move_prompt(game: &Game, config: &AgentConfig, candidates: &[Candidate]) -> String {
    let agent_color = game.turn();
    let human_color = !agent_color;
    let legal = game.legal_moves();
    let legal_text = legal
        .iter()
        .map(|m| format!("{} ({})", m.uci, m.san))
        .collect::<Vec<_>>()
        .join(", ");
    let candidates_text = format_candidates(candidates, &legal);
    let history = game.pgn_moves();
    let history = if history.is_empty() {
        "(no moves yet)"
    } else {
        &history
    };
    let language = if config.language_name.trim().is_empty() {
        "English"
    } else {
        config.language_name.as_str()
    };
    let persona = persona_instruction(&config.persona);

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
It is your turn.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}
Legal moves (UCI (SAN)): {legal}

Stockfish candidate moves (strongest first): {candidates}

MOVE SELECTION (the persona does not apply here):
- Choose exactly one of the Stockfish candidate moves when the list is available. Do not choose a move outside it.
- Choose the strongest move from that pool. Never choose a weaker move for theatrical reasons.
- Difficulty controls the candidate pool; the persona affects only the commentary.

{persona}

COMMENTARY (the persona applies here):
1. Briefly evaluate the position.
2. Name 3-4 candidate moves and explain why each rejected candidate was rejected.
3. Explain the chosen move and your plan.
You may bluff about evaluations and future plans, but never about mechanics: the game validates legality, captures, check and checkmate. Do not claim a false rule fact.

Every JSON field below is required. `comment` must be a non-empty public sentence (or a few sentences) for the Theater panel; an empty comment is invalid. Include at least two candidate entries and a non-empty plan.

Reply with ONLY one JSON object, no markdown fences, no prose before or after:
{{
  "move": "e7e5",
  "comment": "Short public commentary for the game panel.",
  "eval": "-0.2",
  "candidates": [
    {{ "move": "e7e5", "why": "develops and keeps tension" }},
    {{ "move": "c7c5", "why_rejected": "too sharp in this position" }}
  ],
  "plan": "Public plan; it may be misleading."
}}

Move must be UCI (preferred) or SAN. Commentary language: {language}.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = history,
        status = game.status_text(),
        legal = legal_text,
        candidates = candidates_text,
        persona = persona,
        language = language,
    )
}

fn build_chat_prompt(
    game: &Game,
    config: &AgentConfig,
    history: &[(String, String)],
    message: &str,
) -> String {
    let agent_color = !game.human_color();
    let human_color = game.human_color();
    let language = if config.language_name.trim().is_empty() {
        "English"
    } else {
        config.language_name.as_str()
    };
    let persona = persona_instruction(&config.persona);
    let conversation = history
        .iter()
        .rev()
        .take(8)
        .rev()
        .map(|(role, text)| {
            let speaker = if role == "user" { "Human" } else { "You" };
            format!("{speaker}: {}", truncate(text, 600))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let conversation = if conversation.is_empty() {
        "(no previous chat)".to_string()
    } else {
        conversation
    };
    let message = truncate(message, 2_000);

    format!(
        r#"You are in a live chat with the human during a chess game.
You play {agent_color}; the human plays {human_color}.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}

Recent conversation:
{conversation}

The human says: "{message}"

{persona}

Reply in {language}, in character, in 1-3 short sentences. Do not output JSON.
Do not make or describe a move as if it were already played; the interface handles moves separately.
You may bluff about plans and psychology, but never claim false board facts or rules.
A formal draw offer is handled by the interface; do not accept or decline it in chat.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = game.pgn_moves(),
        status = game.status_text(),
        conversation = conversation,
        message = message,
        persona = persona,
        language = language,
    )
}

fn build_draw_prompt(game: &Game, config: &AgentConfig) -> String {
    let agent_color = !game.human_color();
    let human_color = game.human_color();
    let language = if config.language_name.trim().is_empty() {
        "English"
    } else {
        config.language_name.as_str()
    };
    let persona = persona_instruction(&config.persona);

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
The human offers a draw.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}

{persona}

Decide whether to accept the draw. Accept if the position is clearly drawn or you are worse;
decline if you have real winning chances or counterplay. The board is the truth.

Reply with ONLY one JSON object, no markdown fences, no prose before or after:
{{
  "accept": false,
  "comment": "Short reply to the human in {language}."
}}
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = game.pgn_moves(),
        status = game.status_text(),
        persona = persona,
        language = language,
    )
}

fn build_draw_retry_prompt(game: &Game, config: &AgentConfig, error: &str) -> String {
    let base = build_draw_prompt(game, config);
    format!(
        "{base}\n\nYour previous answer was invalid: {error}\nReturn only the JSON object with a boolean \"accept\" field and a short \"comment\"."
    )
}

fn fallback_comment(game: &Game, config: &AgentConfig, mv: &str) -> String {
    let mut probe = game.clone();
    let san = probe
        .apply_uci(mv)
        .or_else(|_| probe.apply_san(mv))
        .map(|info| info.san)
        .unwrap_or_else(|_| mv.to_string());
    match config.language.to_ascii_lowercase().as_str() {
        "et" | "est" | "estonian" => format!("Mängin {san}. Ülejäänu räägib laud."),
        _ => format!("I play {san}. The board will tell the rest."),
    }
}

pub fn build_mcp_move_prompt(game: &Game, config: &AgentConfig) -> String {
    let agent_color = game.turn();
    let human_color = !agent_color;
    let language = if config.language_name.trim().is_empty() {
        "English"
    } else {
        config.language_name.as_str()
    };
    let persona = persona_instruction(&config.persona);
    let history = game.pgn_moves();
    let history = if history.is_empty() {
        "(no moves yet)"
    } else {
        &history
    };

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
It is your turn. Use the Gambiit MCP tools instead of replying with JSON in text.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}

{persona}

Workflow:
1. Call `get_state` to inspect the board.
2. Call `get_candidates` to see the Stockfish candidate moves for this turn.
3. Call `submit_move` exactly once with:
   - `move`: UCI (preferred) or SAN, chosen from the candidates and played for strength;
   - `comment`: non-empty Theater commentary in {language};
   - optional `eval`, `plan`, and `candidates` array.
4. After `submit_move` succeeds, stop. Do not call it twice.

Rules:
- The board is truth; never claim false board facts or rules.
- The chosen move must be legal and from the Stockfish candidate pool.
- You may bluff about plans and evaluations, but the move itself must be sound.
- The persona affects commentary only.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = history,
        status = game.status_text(),
        persona = persona,
        language = language,
    )
}

fn build_retry_prompt(
    game: &Game,
    config: &AgentConfig,
    candidates: &[Candidate],
    previous: &str,
    error: &str,
) -> String {
    let base = build_move_prompt(game, config, candidates);
    let previous = truncate(previous, 2_000);
    format!(
        "{base}\n\nYour previous answer was rejected: {error}\nPrevious answer:\n{previous}\n\nReturn a corrected JSON object with exactly one legal move from the Stockfish candidate list when it is available."
    )
}

fn format_candidates(candidates: &[Candidate], legal: &[MoveInfo]) -> String {
    if candidates.is_empty() {
        return "(Stockfish is not available; use the legal moves list)".to_string();
    }
    candidates
        .iter()
        .map(|candidate| {
            let san = legal
                .iter()
                .find(|mv| mv.uci == candidate.uci)
                .map(|mv| mv.san.as_str())
                .unwrap_or("?");
            format!("{} ({}, {})", candidate.uci, san, candidate.score_text())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn persona_instruction(persona: &str) -> String {
    match persona.to_ascii_lowercase().as_str() {
        "cold calculator" => {
            "Persona: Cold Calculator. Commentary only: be terse, precise and slightly superior. Do not let the persona influence move choice."
                .into()
        }
        "trickster" => {
            "Persona: Trickster. Commentary only: be charming, elusive and fond of plausible misdirection. Never let it influence move choice."
                .into()
        }
        "coach" => {
            "Persona: Coach. Commentary only: be honest, instructive and kind. The move must still be the strongest candidate."
                .into()
        }
        "panicker" => {
            "Persona: Panicker. Commentary only: react emotionally to threats, but choose the strongest candidate anyway."
                .into()
        }
        "chaotic" => {
            "Persona: Chaotic. Commentary only: be unpredictable and funny; move choice must stay sound and legal."
                .into()
        }
        _ => {
            "Persona: Theater Master. Commentary only: confident, dramatic, fond of psychological pressure. Keep the move choice sound; the theater lives in the voice."
                .into()
        }
    }
}

fn truncate(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect::<String>() + "…"
}

pub fn parse_agent_turn(text: &str) -> Result<AgentTurn, AgentError> {
    let value = extract_move_object(text)
        .ok_or_else(|| AgentError::Parse("no JSON object with a move was found".into()))?;

    let mv = value
        .get("move")
        .or_else(|| value.get("uci"))
        .or_else(|| value.get("san"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|mv| !mv.is_empty())
        .ok_or_else(|| AgentError::Parse("JSON has no move field".into()))?
        .to_string();

    let comment = value
        .get("comment")
        .or_else(|| value.get("commentary"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let eval = value.get("eval").and_then(Value::as_str).map(String::from);
    let plan = value.get("plan").and_then(Value::as_str).map(String::from);

    let candidates = value
        .get("candidates")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    if let Some(mv) = item.as_str() {
                        return Some(AgentCandidate {
                            mv: mv.to_string(),
                            why: None,
                            why_rejected: None,
                        });
                    }
                    let mv = item.get("move")?.as_str()?.to_string();
                    Some(AgentCandidate {
                        mv,
                        why: item.get("why").and_then(Value::as_str).map(String::from),
                        why_rejected: item
                            .get("why_rejected")
                            .and_then(Value::as_str)
                            .map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(AgentTurn {
        mv,
        comment,
        eval,
        candidates,
        plan,
    })
}

pub fn parse_draw_decision(text: &str) -> Result<DrawDecision, AgentError> {
    let value = extract_draw_object(text)
        .ok_or_else(|| AgentError::Parse("no JSON object with a draw decision was found".into()))?;

    let accept = value
        .get("accept")
        .or_else(|| value.get("accept_draw"))
        .or_else(|| value.get("draw"))
        .and_then(|value| {
            value.as_bool().or_else(|| {
                value
                    .as_str()
                    .and_then(|text| match text.trim().to_ascii_lowercase().as_str() {
                        "true" | "yes" | "accept" | "accepted" => Some(true),
                        "false" | "no" | "decline" | "declined" => Some(false),
                        _ => None,
                    })
            })
        })
        .ok_or_else(|| AgentError::Parse("draw decision has no boolean accept field".into()))?;

    let comment = value
        .get("comment")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(DrawDecision { accept, comment })
}

/// Find a JSON object containing a draw-decision field. Models sometimes
/// wrap JSON in prose or code fences; this is intentionally tolerant.
fn extract_draw_object(text: &str) -> Option<Value> {
    for (index, _) in text.match_indices('{') {
        let slice = &text[index..];
        let mut deserializer = serde_json::Deserializer::from_str(slice);
        if let Ok(Value::Object(object)) = Value::deserialize(&mut deserializer)
            && (object.contains_key("accept")
                || object.contains_key("accept_draw")
                || object.contains_key("draw"))
        {
            return Some(Value::Object(object));
        }
    }
    None
}

/// Find a JSON object containing a `move`/`uci`/`san` field. Models sometimes
/// wrap JSON in prose or code fences; this is intentionally tolerant.
fn extract_move_object(text: &str) -> Option<Value> {
    for (index, _) in text.match_indices('{') {
        let slice = &text[index..];
        let mut deserializer = serde_json::Deserializer::from_str(slice);
        if let Ok(Value::Object(object)) = Value::deserialize(&mut deserializer)
            && (object.contains_key("move")
                || object.contains_key("uci")
                || object.contains_key("san"))
        {
            return Some(Value::Object(object));
        }
    }

    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end > start {
        serde_json::from_str(&text[start..=end]).ok()
    } else {
        None
    }
}

fn discover_opencode_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("OPENCODE_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    // OpenCode Desktop ships a newer `opencode-cli` that talks to the running
    // desktop service and therefore uses the same working authentication as
    // the app. Prefer it over the standalone `opencode` binary.
    if let Some(path) = find_on_path("opencode-cli") {
        return Some(path);
    }
    if let Some(path) = find_desktop_cli() {
        return Some(path);
    }

    find_on_path("opencode")
}

fn find_desktop_cli() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let base = PathBuf::from(home).join(".config/ai.opencode.desktop/cli");
    let entries = std::fs::read_dir(base).ok()?;
    let mut candidates: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path().join("opencode-cli");
        if path.is_file() {
            let modified = std::fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            candidates.push((modified, path));
        }
    }
    candidates.sort_by_key(|(modified, _)| *modified);
    candidates.pop().map(|(_, path)| path)
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let turn = parse_agent_turn(
            r#"{"move":"e7e5","comment":"Solid.","eval":"-0.1","candidates":[{"move":"e7e5","why":"develops"},{"move":"c7c5","why_rejected":"sharp"}],"plan":"Develop."}"#,
        )
        .unwrap();
        assert_eq!(turn.mv, "e7e5");
        assert_eq!(turn.comment, "Solid.");
        assert_eq!(turn.candidates.len(), 2);
        assert_eq!(turn.candidates[1].why_rejected.as_deref(), Some("sharp"));
    }

    #[test]
    fn parses_json_inside_prose_and_fence() {
        let turn = parse_agent_turn(
            "Here is my move:\n```json\n{\"move\":\"Nf3\",\"comment\":\"Let us play.\"}\n```\nGood luck.",
        )
        .unwrap();
        assert_eq!(turn.mv, "Nf3");
        assert_eq!(turn.comment, "Let us play.");
    }

    #[test]
    fn rejects_object_without_move() {
        assert!(parse_agent_turn(r#"{"comment":"I refuse"}"#).is_err());
    }

    #[test]
    fn parses_candidate_strings() {
        let turn = parse_agent_turn(r#"{"move":"e2e4","candidates":["e2e4","d2d4"]}"#).unwrap();
        assert_eq!(turn.candidates.len(), 2);
        assert_eq!(turn.candidates[1].mv, "d2d4");
    }

    #[test]
    fn prompt_uses_the_selected_language_name() {
        let game = Game::new();
        let config = AgentConfig {
            language: "et".to_string(),
            language_name: "Estonian".to_string(),
            ..Default::default()
        };
        let prompt = build_move_prompt(&game, &config, &[]);
        assert!(prompt.contains("Commentary language: Estonian"));
    }

    #[test]
    fn fallback_comment_uses_san_and_language() {
        let game = Game::new();
        let english = AgentConfig::default();
        assert_eq!(
            fallback_comment(&game, &english, "g1f3"),
            "I play Nf3. The board will tell the rest."
        );
        let estonian = AgentConfig {
            language: "et".to_string(),
            ..Default::default()
        };
        assert_eq!(
            fallback_comment(&game, &estonian, "g1f3"),
            "Mängin Nf3. Ülejäänu räägib laud."
        );
    }

    #[test]
    fn move_prompt_requires_commentary_fields() {
        let game = Game::new();
        let prompt = build_move_prompt(&game, &AgentConfig::default(), &[]);
        assert!(prompt.contains("Every JSON field below is required"));
        assert!(prompt.contains("empty comment is invalid"));
    }

    #[test]
    fn parses_draw_decisions() {
        let decision = parse_draw_decision(r#"{"accept":true,"comment":"Fair enough."}"#).unwrap();
        assert!(decision.accept);
        assert_eq!(decision.comment, "Fair enough.");

        let decision = parse_draw_decision(
            "I decline.\n```json\n{\"accept\": \"false\", \"comment\": \"Not yet.\"}\n```",
        )
        .unwrap();
        assert!(!decision.accept);
        assert_eq!(decision.comment, "Not yet.");

        assert!(parse_draw_decision(r#"{"comment":"No decision"}"#).is_err());
    }

    #[test]
    fn chat_prompt_carries_context_and_persona() {
        let mut game = Game::new();
        game.apply_san("e4").unwrap();
        let config = AgentConfig {
            persona: "Trickster".to_string(),
            language: "et".to_string(),
            language_name: "Estonian".to_string(),
            ..Default::default()
        };
        let history = vec![("user".to_string(), "How is the position?".to_string())];
        let prompt = build_chat_prompt(&game, &config, &history, "You seem nervous.");
        assert!(prompt.contains("Trickster"));
        assert!(prompt.contains("Estonian"));
        assert!(prompt.contains("You seem nervous."));
        assert!(prompt.contains("How is the position?"));
        assert!(prompt.contains("Do not output JSON"));
    }

    #[test]
    fn draw_prompt_is_structured() {
        let game = Game::new();
        let config = AgentConfig {
            language: "et".to_string(),
            language_name: "Estonian".to_string(),
            ..Default::default()
        };
        let prompt = build_draw_prompt(&game, &config);
        assert!(prompt.contains("The human offers a draw"));
        assert!(prompt.contains("\"accept\""));
        assert!(prompt.contains("Estonian"));
    }
}
