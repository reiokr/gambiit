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
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
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
pub struct AgentConfig {
    pub model: String,
    pub persona: String,
    pub language: String,
    pub timeout_secs: u64,
    pub retries: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "ollama/gemma4".to_string(),
            persona: "Theater Master".to_string(),
            language: "en".to_string(),
            timeout_secs: 180,
            retries: 1,
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
        let binary = std::env::var_os("OPENCODE_BIN")
            .map(PathBuf::from)
            .or_else(|| find_on_path("opencode"))
            .ok_or(AgentError::OpenCodeNotFound)?;
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
        let mut command = Command::new(&self.binary);
        command
            .arg("run")
            .arg("--pure")
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
            if let Some(tokens) = part
                .get("tokens")
                .and_then(|tokens| tokens.get("total"))
                .and_then(Value::as_u64)
            {
                parsed.tokens = Some(tokens);
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

            let output = self
                .client
                .run(&self.config, &prompt, self.session_id.as_deref())?;
            if output.session_id.is_some() {
                self.session_id = output.session_id.clone();
            }
            self.last_raw = Some(output.text.clone());

            match parse_agent_turn(&output.text) {
                Ok(turn) => match validate(&turn) {
                    Ok(()) => return Ok((turn, output)),
                    Err(error) => {
                        last_error = Some(error);
                    }
                },
                Err(error) => {
                    last_error = Some(error.to_string());
                }
            }
        }

        Err(AgentError::InvalidMove(last_error.unwrap_or_else(|| {
            "agent did not return a valid move".into()
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
    let language = if config.language.eq_ignore_ascii_case("et") {
        "Estonian"
    } else {
        "English"
    };
    let persona = persona_instruction(&config.persona);

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
It is your turn.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}
Legal moves (UCI (SAN)): {legal}

Stockfish candidate moves (optional, strongest first): {candidates}
Choose one legal move. You may choose a move outside the candidate list if you have a concrete reason.

{persona}

Commentary requirements (AGENTS.md protocol):
1. Briefly evaluate the position.
2. Name 3-4 candidate moves and explain why each rejected candidate was rejected.
3. Explain the chosen move and your plan.
You may bluff about evaluations and future plans, but never about mechanics: the game validates legality, captures, check and checkmate. Do not claim a false rule fact.

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
        "{base}\n\nYour previous answer was rejected: {error}\nPrevious answer:\n{previous}\n\nReturn a corrected JSON object with exactly one legal move."
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
            "Persona: Cold Calculator. Be terse, precise and slightly superior. No jokes.".into()
        }
        "trickster" => {
            "Persona: Trickster. Be charming, elusive and fond of plausible misdirection. Never wink at the audience."
                .into()
        }
        "coach" => {
            "Persona: Coach. Be honest, instructive and kind. Explain the chess, do not deceive.".into()
        }
        "panicker" => {
            "Persona: Panicker. React emotionally to threats, but keep the analysis accurate.".into()
        }
        "chaotic" => {
            "Persona: Chaotic. Be unpredictable and funny, but still play legal, intentional chess.".into()
        }
        _ => {
            "Persona: Theater Master. Treat the game as a performance: confident, dramatic, and fond of psychological pressure. Keep the chess sound."
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
}
