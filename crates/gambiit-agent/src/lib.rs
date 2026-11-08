//! OpenCode prompt transport and agent response contract.
//!
//! The agent is allowed to be theatrical. The game is not: every move returned
//! here is validated against `gambiit-core` before it reaches the board.

use gambiit_core::{Game, MoveInfo, color_name};
use gambiit_engine::Candidate;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
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
    #[error("could not prepare the Gambiit agent workspace: {0}")]
    Workspace(String),
    #[error("{CANCELLED_MESSAGE}")]
    Cancelled,
}

/// Stable error text used by the UI to distinguish a user cancellation from a
/// real agent failure.
pub const CANCELLED_MESSAGE: &str = "agent request was cancelled";

/// Cancellation handle shared by the Tauri state and the running agent request.
///
/// `begin` installs a fresh flag for a new operation, so a late `cancel` can
/// never poison the next request. `cancel` only needs the handle, not the game
/// session lock, which is important because agent moves hold that lock while
/// the child process runs.
#[derive(Clone, Default)]
pub struct AgentCancel {
    current: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl AgentCancel {
    pub fn begin(&self) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut current) = self.current.lock() {
            *current = Some(flag.clone());
        }
        flag
    }

    pub fn cancel(&self) {
        if let Ok(current) = self.current.lock()
            && let Some(flag) = current.as_ref()
        {
            flag.store(true, Ordering::Relaxed);
        }
    }
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
    /// Optional bounded move-style guidance from the selected persona.
    pub style_instruction: String,
    /// Optional custom persona prompt; overrides the built-in persona text.
    pub persona_prompt: String,
    /// Optional bluff-intensity guidance for evaluations and plans.
    pub bluff_instruction: String,
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
            style_instruction: String::new(),
            persona_prompt: String::new(),
            bluff_instruction: String::new(),
        }
    }
}

fn bluff_section(config: &AgentConfig) -> String {
    let bluff = config.bluff_instruction.trim();
    if bluff.is_empty() {
        String::new()
    } else {
        format!("{bluff}\n\n")
    }
}

fn persona_instruction_for(config: &AgentConfig) -> String {
    if config.persona_prompt.trim().is_empty() {
        persona_instruction(&config.persona)
    } else {
        config.persona_prompt.trim().to_string()
    }
}

fn terminate_child(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        if pid > 0 {
            // Negative pid targets the process group created above.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
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

static RESTRICTED_WORKSPACE: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// OpenCode project config for the Gambiit agent.
///
/// The agent may use the Gambiit MCP server when one is added to this object,
/// but every built-in shell, file, web and task tool is denied. Gambiit adds
/// the MCP server itself in `write_mcp_config`.
pub fn restricted_agent_config() -> Value {
    json!({
        "default_agent": "gambiit",
        "agent": {
            "gambiit": {
                "description": "Gambiit chess opponent. Use only the Gambiit chess tools; never shell, files, web or other agents.",
                "mode": "primary",
                "tools": {
                    "bash": false,
                    "edit": false,
                    "write": false,
                    "read": false,
                    "webfetch": false,
                    "websearch": false,
                    "task": false,
                    "todowrite": false,
                    "todoread": false,
                    "list": false,
                    "glob": false,
                    "grep": false,
                    "patch": false,
                    "multiedit": false,
                    "apply_patch": false,
                    "lsp-diagnostics": false,
                    "lsp-hover": false
                },
                "permission": {
                    "bash": "deny",
                    "edit": "deny",
                    "write": "deny",
                    "read": "deny",
                    "webfetch": "deny",
                    "websearch": "deny",
                    "task": "deny"
                }
            }
        }
    })
}

fn restricted_workspace() -> Result<PathBuf, AgentError> {
    let result = RESTRICTED_WORKSPACE.get_or_init(|| {
        let directory = std::env::temp_dir().join(format!("gambiit-agent-{}", std::process::id()));
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|error| error.to_string())?;
        }
        let path = directory.join("opencode.json");
        let temporary = directory.join(".opencode.json.tmp");
        let contents = serde_json::to_string_pretty(&restricted_agent_config())
            .map_err(|error| error.to_string())?;
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
                .map_err(|error| error.to_string())?;
            file.write_all(contents.as_bytes())
                .map_err(|error| error.to_string())?;
            file.sync_all().ok();
        }
        fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
        Ok(directory)
    });
    result.clone().map_err(AgentError::Workspace)
}

impl OpenCodeClient {
    pub fn new(timeout_secs: u64) -> Result<Self, AgentError> {
        let binary = discover_opencode_binary().ok_or(AgentError::OpenCodeNotFound)?;
        Ok(Self {
            binary,
            timeout_secs,
        })
    }

    /// Construct a client for an explicit binary. Used by cancellation tests
    /// and by embedders that already resolved the executable.
    pub fn with_binary(binary: PathBuf, timeout_secs: u64) -> Self {
        Self {
            binary,
            timeout_secs,
        }
    }

    pub fn binary(&self) -> &PathBuf {
        &self.binary
    }

    /// Run one prompt through the OpenCode CLI and collect the text events.
    ///
    /// The prompt runs in a private temporary OpenCode project using the
    /// restricted `gambiit` agent: no shell, file, web or task tools.
    /// `--format json` emits NDJSON; we parse the assistant text parts and the
    /// session id, ignoring unrelated events.
    pub fn run(
        &self,
        config: &AgentConfig,
        prompt: &str,
        session: Option<&str>,
    ) -> Result<OpenCodeOutput, AgentError> {
        self.run_with_cancel(config, prompt, session, None)
    }

    pub fn run_with_cancel(
        &self,
        config: &AgentConfig,
        prompt: &str,
        session: Option<&str>,
        cancel: Option<&AtomicBool>,
    ) -> Result<OpenCodeOutput, AgentError> {
        let workspace = restricted_workspace()?;
        self.run_with_agent(config, prompt, session, &[], Some(&workspace), cancel)
    }

    /// Run an MCP turn in the restricted agent project prepared by the caller.
    fn run_mcp(
        &self,
        config: &AgentConfig,
        prompt: &str,
        session: Option<&str>,
        working_dir: &Path,
        cancel: Option<&AtomicBool>,
    ) -> Result<OpenCodeOutput, AgentError> {
        self.run_with_agent(config, prompt, session, &[], Some(working_dir), cancel)
    }

    fn run_with_agent(
        &self,
        config: &AgentConfig,
        prompt: &str,
        session: Option<&str>,
        extra_env: &[(String, String)],
        working_dir: Option<&Path>,
        cancel: Option<&AtomicBool>,
    ) -> Result<OpenCodeOutput, AgentError> {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(AgentError::Cancelled);
        }
        let mut command = Command::new(&self.binary);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // A dedicated process group lets cancellation terminate helpers such
            // as the shell and its child processes, not just the parent.
            command.process_group(0);
        }
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
            .stderr(Stdio::piped())
            .arg("--agent")
            .arg("gambiit");
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
            if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                terminate_child(&mut child);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(AgentError::Cancelled);
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() > Duration::from_secs(self.timeout_secs) {
                terminate_child(&mut child);
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
    cancel: Option<Arc<AtomicBool>>,
}

impl AgentSession {
    pub fn new(config: AgentConfig) -> Result<Self, AgentError> {
        let client = OpenCodeClient::new(config.timeout_secs)?;
        Ok(Self {
            client,
            config,
            session_id: None,
            last_raw: None,
            cancel: None,
        })
    }

    /// Attach the cancellation flag for the current operation.
    pub fn set_cancel(&mut self, cancel: Option<Arc<AtomicBool>>) {
        self.cancel = cancel;
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

            let output = match self.client.run_with_cancel(
                &self.config,
                &prompt,
                self.session_id.as_deref(),
                self.cancel.as_deref(),
            ) {
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
        let output = self.client.run_with_cancel(
            &self.config,
            &prompt,
            self.session_id.as_deref(),
            self.cancel.as_deref(),
        )?;
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
    /// Gambiit MCP server. The model is expected to call the relevant tool.
    pub fn request_mcp(
        &mut self,
        prompt: &str,
        working_dir: &Path,
    ) -> Result<OpenCodeOutput, AgentError> {
        let output = self.client.run_mcp(
            &self.config,
            prompt,
            self.session_id.as_deref(),
            working_dir,
            self.cancel.as_deref(),
        )?;
        if output.session_id.is_some() {
            self.session_id = output.session_id.clone();
        }
        self.last_raw = Some(output.text.clone());
        Ok(output)
    }

    /// Send one plain-text prompt and return the model's trimmed reply. Used by
    /// the honest coach, which has its own prompt and never touches the board.
    pub fn request_text(&mut self, prompt: &str) -> Result<(String, OpenCodeOutput), AgentError> {
        let output = self.client.run_with_cancel(
            &self.config,
            prompt,
            self.session_id.as_deref(),
            self.cancel.as_deref(),
        )?;
        if output.session_id.is_some() {
            self.session_id = output.session_id.clone();
        }
        let reply: String = output.text.trim().chars().take(8_000).collect();
        if reply.is_empty() {
            return Err(AgentError::EmptyResponse);
        }
        Ok((reply, output))
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
            let output = self.client.run_with_cancel(
                &self.config,
                &prompt,
                self.session_id.as_deref(),
                self.cancel.as_deref(),
            )?;
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
    let persona = persona_instruction_for(config);
    let style = config.style_instruction.trim();
    let style_section = if style.is_empty() {
        String::new()
    } else {
        format!("{style}\n\n")
    };
    let candidates_heading = if style.is_empty() {
        "Stockfish candidate moves (strongest first)"
    } else {
        "Stockfish candidate moves (the first is your persona's style preference; all are within the allowed strength margin)"
    };
    let selection_rules = if style.is_empty() {
        "Choose the strongest move from that pool. Never choose a weaker move for theatrical reasons."
    } else {
        "The first candidate is the persona's style preference and is within the chosen strength margin. Play it unless you have a concrete reason it is unsound. Do not choose a move outside the candidate pool."
    };
    let bluff_section = bluff_section(config);

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
It is your turn.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}
Legal moves (UCI (SAN)): {legal}

{style_section}{candidates_heading}: {candidates}

MOVE SELECTION:
- Choose exactly one of the Stockfish candidate moves when the list is available. Do not choose a move outside it.
- {selection_rules}
- Difficulty controls the candidate pool; the persona affects only the bounded style and commentary.

{persona}

{bluff_section}COMMENTARY (stay in character):
1. Briefly evaluate the position.
2. Name 3-4 candidate moves and explain why each rejected candidate was rejected.
3. Explain the chosen move and your plan.
Never lie about mechanics: the game validates legality, captures, check and checkmate. Do not claim a false rule fact.

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
  "plan": "Public plan; stay in character."
}}

`eval` may be a number (e.g. +0.3), a mate claim (#3), or one of: winning, losing, equal, better, worse. `eval` is from your side's perspective.
Move must be UCI (preferred) or SAN. Commentary language: {language}.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = history,
        status = game.status_text(),
        legal = legal_text,
        style_section = style_section,
        candidates_heading = candidates_heading,
        candidates = candidates_text,
        selection_rules = selection_rules,
        persona = persona,
        bluff_section = bluff_section,
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
    let persona = persona_instruction_for(config);
    let bluff_section = bluff_section(config);
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

{bluff_section}Reply in {language}, in character, in 1-3 short sentences. Do not output JSON.
Do not make or describe a move as if it were already played; the interface handles moves separately.
Never claim false board facts or rules.
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
        bluff_section = bluff_section,
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
    let persona = persona_instruction_for(config);
    let bluff_section = bluff_section(config);

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
The human offers a draw.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}

{persona}

{bluff_section}Decide whether to accept the draw. Accept if the position is clearly drawn or you are worse;
decline if you have real winning chances or counterplay. The board is the truth.
The draw decision itself is not a bluff; only the comment may be in character.

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
        bluff_section = bluff_section,
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

pub fn build_mcp_move_prompt(
    game: &Game,
    config: &AgentConfig,
    candidates: &[Candidate],
) -> String {
    let agent_color = game.turn();
    let human_color = !agent_color;
    let legal = game.legal_moves();
    let candidates_text = format_candidates(candidates, &legal);
    let language = if config.language_name.trim().is_empty() {
        "English"
    } else {
        config.language_name.as_str()
    };
    let persona = persona_instruction_for(config);
    let style = config.style_instruction.trim();
    let style_section = if style.is_empty() {
        String::new()
    } else {
        format!("{style}\n\n")
    };
    let bluff_section = bluff_section(config);
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

Stockfish candidate moves (the first is your persona's style preference; all are within the allowed strength margin): {candidates}

{style_section}{persona}

{bluff_section}Fast path:
1. The board state and candidate moves are already included above. Do not call `get_state` or `get_candidates` mechanically.
2. If you need to re-check something, you may call `get_state`, `get_candidates`, `get_clock`, or `get_analysis` (the latest post-game summary plus the current engine read).
3. Call `submit_move` exactly once with:
   - `move`: UCI (preferred) or SAN, chosen from the candidates and played for strength;
   - `comment`: non-empty Theater commentary in {language};
   - optional `eval`, `plan`, and `candidates` array.
4. After `submit_move` succeeds, stop. Do not call it twice.

Rules:
- The board is truth; never claim false board facts or rules.
- The chosen move must be legal and from the Stockfish candidate pool.
- The first candidate is the persona's style preference; play it unless you have a concrete reason it is unsound.
- The move itself must be sound; bluffing never extends to board facts or rules.
- `eval` may be a number (e.g. +0.3), a mate claim (#3), or one of: winning, losing, equal, better, worse. `eval` is from your side's perspective.
- The persona affects bounded style and commentary only.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = history,
        status = game.status_text(),
        style_section = style_section,
        candidates = candidates_text,
        persona = persona,
        bluff_section = bluff_section,
        language = language,
    )
}

pub fn build_mcp_chat_prompt(
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
    let persona = persona_instruction_for(config);
    let bluff_section = bluff_section(config);
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
Use the Gambiit MCP tools instead of replying with JSON or prose as your final answer.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}

Recent conversation:
{conversation}

The human says: "{message}"

{persona}

{bluff_section}Fast path:
1. The board state and conversation are already above. Do not call `get_state` mechanically.
2. If you need an honest engine read, call `get_analysis` once. You may call `get_state` or `get_clock` if needed.
3. Call `submit_chat` exactly once with `message`: your in-character reply in {language}, 1-3 short sentences.
4. After `submit_chat` succeeds, stop. Do not call it twice. Do not call `submit_move` or `submit_draw`.

Rules:
- The board is truth; never claim false board facts or rules.
- Chat never changes the board. A formal draw offer is handled separately.
- Stay in character. Bluffing follows the bluff instruction and never extends to rules or board facts.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = game.pgn_moves(),
        status = game.status_text(),
        conversation = conversation,
        message = message,
        persona = persona,
        bluff_section = bluff_section,
        language = language,
    )
}

pub fn build_mcp_draw_prompt(game: &Game, config: &AgentConfig) -> String {
    let agent_color = !game.human_color();
    let human_color = game.human_color();
    let language = if config.language_name.trim().is_empty() {
        "English"
    } else {
        config.language_name.as_str()
    };
    let persona = persona_instruction_for(config);
    let bluff_section = bluff_section(config);

    format!(
        r#"You are playing chess as {agent_color} against a human ({human_color}).
The human offers a draw.
Use the Gambiit MCP tools instead of replying with JSON or prose as your final answer.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}

{persona}

{bluff_section}Fast path:
1. The board state is already above. Call `get_analysis` once if you want an honest engine read of the position.
2. Decide whether to accept the draw. Accept if the position is clearly drawn or you are worse; decline if you have real winning chances or counterplay. The board is the truth.
3. Call `submit_draw` exactly once with `accept` (boolean) and `comment` (short reply in {language}).
4. After `submit_draw` succeeds, stop. Do not call it twice. Do not call `submit_move` or `submit_chat`.

Rules:
- The draw decision itself is not a bluff; only the comment may be in character.
- Never claim false board facts or rules.
- The board is truth; the opponent's draw offer is handled by the interface.
"#,
        agent_color = color_name(agent_color),
        human_color = color_name(human_color),
        fen = game.fen(),
        history = game.pgn_moves(),
        status = game.status_text(),
        persona = persona,
        bluff_section = bluff_section,
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
            "Persona: Cold Calculator. Be terse, precise and slightly superior. Always play the strongest available candidate; style never overrides strength."
                .into()
        }
        "trickster" => {
            "Persona: Trickster. Be charming, elusive and fond of plausible misdirection. When a style preference is listed, you may play it inside the allowed margin; otherwise choose the strongest candidate. Never invent a move outside the pool."
                .into()
        }
        "coach" => {
            "Persona: Coach. Be honest, instructive and kind. Choose a principled, sound move; a style preference must still be clean and inside the margin."
                .into()
        }
        "panicker" => {
            "Persona: Panicker. React emotionally to threats, but keep the move sound; prefer the cautious style candidate when it is inside the margin."
                .into()
        }
        "chaotic" => {
            "Persona: Chaotic. Be unpredictable and funny. Move choice must stay sound and legal; offbeat candidates are welcome only inside the margin."
                .into()
        }
        _ => {
            "Persona: Theater Master. Confident, dramatic, fond of psychological pressure. Prefer the style candidate when it stays sound; the theater lives in the voice and the bounded style."
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
    fn cancellation_handle_targets_only_the_current_request() {
        let cancel = AgentCancel::default();
        let first = cancel.begin();
        assert!(!first.load(Ordering::Relaxed));
        cancel.cancel();
        assert!(first.load(Ordering::Relaxed));

        let second = cancel.begin();
        assert!(!second.load(Ordering::Relaxed));
        assert_eq!(AgentError::Cancelled.to_string(), CANCELLED_MESSAGE);
    }

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
    fn style_instruction_is_rendered_in_prompts() {
        let game = Game::new();
        let config = AgentConfig {
            style_instruction: "Persona move style: trickster.".to_string(),
            ..Default::default()
        };
        let candidates = vec![Candidate {
            uci: "e2e4".to_string(),
            score_cp: Some(24),
            mate_in: None,
            depth: 10,
            multipv: 1,
            pv: Vec::new(),
        }];
        let prompt = build_move_prompt(&game, &config, &candidates);
        assert!(prompt.contains("Persona move style: trickster."));
        assert!(prompt.contains("style preference"));
        let mcp = build_mcp_move_prompt(&game, &config, &candidates);
        assert!(mcp.contains("Persona move style: trickster."));
        assert!(mcp.contains("style preference"));
    }

    #[test]
    fn bluff_instruction_is_rendered_in_prompts() {
        let game = Game::new();
        let config = AgentConfig {
            bluff_instruction: "BLUFF INTENSITY: 100/100 (unreliable narrator).".to_string(),
            ..Default::default()
        };
        let marker = "BLUFF INTENSITY: 100/100";
        assert!(build_move_prompt(&game, &config, &[]).contains(marker));
        assert!(build_chat_prompt(&game, &config, &[], "Hello").contains(marker));
        assert!(build_draw_prompt(&game, &config).contains(marker));
        assert!(build_mcp_move_prompt(&game, &config, &[]).contains(marker));
    }

    #[test]
    fn built_in_personas_allow_bounded_style() {
        for persona in [
            "Theater Master",
            "Cold Calculator",
            "Trickster",
            "Panicker",
            "Coach",
            "Chaotic",
        ] {
            let text = persona_instruction(persona);
            assert!(!text.contains("Commentary only"), "{persona}: {text}");
            assert!(
                text.contains("candidate") || text.contains("strongest") || text.contains("style"),
                "{persona}: {text}"
            );
        }
    }

    #[test]
    fn custom_persona_prompt_overrides_the_built_in_voice() {
        let game = Game::new();
        let config = AgentConfig {
            persona: "Custom".to_string(),
            persona_prompt: "Persona: Velvet Cipher. Theatricality 9/10.".to_string(),
            ..Default::default()
        };
        let prompt = build_move_prompt(&game, &config, &[]);
        assert!(prompt.contains("Velvet Cipher"));
        assert!(!prompt.contains("Persona: Theater Master"));
    }

    #[test]
    fn mcp_prompt_includes_candidates_and_fast_path() {
        let game = Game::new();
        let candidates = vec![Candidate {
            uci: "e2e4".to_string(),
            score_cp: Some(24),
            mate_in: None,
            depth: 10,
            multipv: 1,
            pv: Vec::new(),
        }];
        let prompt = build_mcp_move_prompt(&game, &AgentConfig::default(), &candidates);
        assert!(prompt.contains("Stockfish candidate moves"));
        assert!(prompt.contains("e2e4"));
        assert!(prompt.contains("Fast path"));
        assert!(prompt.contains("Do not call `get_state` or `get_candidates` mechanically"));
    }

    #[test]
    fn restricted_agent_denies_builtin_tools() {
        let config = restricted_agent_config();
        assert_eq!(config["default_agent"], "gambiit");
        let tools = &config["agent"]["gambiit"]["tools"];
        for tool in [
            "bash",
            "edit",
            "write",
            "read",
            "webfetch",
            "websearch",
            "task",
            "todowrite",
            "todoread",
            "list",
            "glob",
            "grep",
        ] {
            assert_eq!(tools[tool], false, "{tool} must be disabled");
        }
        assert_eq!(config["agent"]["gambiit"]["permission"]["bash"], "deny");
    }

    #[test]
    fn restricted_workspace_writes_a_private_agent_config() {
        let directory = restricted_workspace().unwrap();
        let path = directory.join("opencode.json");
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["default_agent"], "gambiit");
        assert_eq!(value["agent"]["gambiit"]["tools"]["bash"], false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn mcp_chat_prompt_uses_submit_chat() {
        let game = Game::new();
        let config = AgentConfig::default();
        let history = vec![
            ("user".to_string(), "Hello.".to_string()),
            ("agent".to_string(), "Welcome.".to_string()),
        ];
        let prompt = build_mcp_chat_prompt(&game, &config, &history, "How is the position?");
        assert!(prompt.contains("submit_chat"));
        assert!(prompt.contains("Do not call `submit_move` or `submit_draw`"));
        assert!(prompt.contains("get_analysis"));
        assert!(prompt.contains("How is the position?"));
    }

    #[test]
    fn mcp_draw_prompt_uses_submit_draw() {
        let game = Game::new();
        let prompt = build_mcp_draw_prompt(&game, &AgentConfig::default());
        assert!(prompt.contains("submit_draw"));
        assert!(prompt.contains("The draw decision itself is not a bluff"));
        assert!(prompt.contains("get_analysis"));
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
