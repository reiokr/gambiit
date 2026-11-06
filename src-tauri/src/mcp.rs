//! Local MCP bridge and stdio MCP server for the Gambiit agent.
//!
//! The desktop app owns a small TCP bridge on localhost. OpenCode starts the
//! same binary with `--mcp-server`; that process speaks MCP over stdio and
//! forwards tool calls to the bridge. The bridge only exposes the current
//! agent turn and never lets the model mutate the game directly.

use gambiit_agent::AgentCandidate;
use gambiit_core::{Game, MoveInfo, color_key};
use gambiit_engine::Candidate;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    env,
    io::{self, BufRead, BufReader, BufWriter, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process,
    sync::Arc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnKind {
    Move,
    Chat,
    Draw,
}

impl TurnKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Move => "move",
            Self::Chat => "chat",
            Self::Draw => "draw",
        }
    }
}

pub struct TurnContext {
    pub kind: TurnKind,
    pub game: Game,
    pub legal: Vec<MoveInfo>,
    pub candidates: Vec<Candidate>,
    pub clock: Option<Value>,
    /// Latest post-game analysis summary, when the game has been analyzed.
    pub analysis: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct MoveSubmission {
    pub mv: String,
    pub comment: String,
    pub eval: Option<String>,
    pub candidates: Vec<AgentCandidate>,
    pub plan: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatSubmission {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DrawSubmission {
    pub accept: bool,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub enum Submission {
    Move(MoveSubmission),
    Chat(ChatSubmission),
    Draw(DrawSubmission),
}

struct BridgeState {
    turn: Option<TurnContext>,
    submission: Option<Submission>,
}

fn candidate_values(turn: &TurnContext) -> Vec<Value> {
    turn.candidates
        .iter()
        .map(|candidate| {
            let san = turn
                .legal
                .iter()
                .find(|mv| mv.uci == candidate.uci)
                .map(|mv| mv.san.as_str())
                .unwrap_or("?");
            json!({
                "uci": candidate.uci,
                "san": san,
                "score": candidate.score_text(),
                "depth": candidate.depth,
            })
        })
        .collect()
}

pub struct Bridge {
    addr: SocketAddr,
    token: String,
    state: Arc<Mutex<BridgeState>>,
}

#[derive(Debug, Deserialize)]
struct WireRequest {
    token: String,
    method: String,
    #[serde(default)]
    arguments: Value,
}

impl Bridge {
    pub fn url(&self) -> String {
        self.addr.to_string()
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn begin(&self, turn: TurnContext) {
        let mut state = self.state.lock();
        state.turn = Some(turn);
        state.submission = None;
    }

    pub fn clear(&self) {
        let mut state = self.state.lock();
        state.turn = None;
        state.submission = None;
    }

    pub fn take_submission(&self) -> Option<Submission> {
        self.state.lock().submission.take()
    }

    fn dispatch(&self, method: &str, arguments: &Value) -> Result<Value, String> {
        let mut state = self.state.lock();
        match method {
            "state" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                Ok(json!({
                    "fen": turn.game.fen(),
                    "turn": color_key(turn.game.turn()),
                    "humanColor": color_key(turn.game.human_color()),
                    "agentColor": color_key(!turn.game.human_color()),
                    "turnKind": turn.kind.as_str(),
                    "status": turn.game.status_text(),
                    "gameOver": turn.game.status().is_over(),
                    "history": turn.game.pgn_moves(),
                    "clock": turn.clock,
                }))
            }
            "legal_moves" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                serde_json::to_value(&turn.legal).map_err(|error| error.to_string())
            }
            "candidates" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                Ok(Value::Array(candidate_values(turn)))
            }
            "analysis" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                let best = candidate_values(turn).into_iter().next();
                Ok(json!({
                    "available": turn.analysis.is_some(),
                    "summary": turn.analysis.clone().unwrap_or(Value::Null),
                    "position": {
                        "fen": turn.game.fen(),
                        "turn": color_key(turn.game.turn()),
                        "status": turn.game.status_text(),
                        "score": turn.candidates.first().map(|candidate| candidate.score_text()),
                        "perspective": color_key(turn.game.turn()),
                        "best": best,
                        "candidates": candidate_values(turn),
                    }
                }))
            }
            "clock" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                Ok(turn.clock.clone().unwrap_or(Value::Null))
            }
            "submit" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                if turn.kind != TurnKind::Move {
                    return Err(format!(
                        "This is a {} turn. Call {} instead.",
                        turn.kind.as_str(),
                        match turn.kind {
                            TurnKind::Chat => "submit_chat",
                            TurnKind::Draw => "submit_draw",
                            TurnKind::Move => "submit_move",
                        }
                    ));
                }
                if state.submission.is_some() {
                    return Err("A response has already been submitted for this turn.".to_string());
                }
                let raw_move = arguments
                    .get("move")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "submit_move requires a non-empty `move`.".to_string())?;
                let comment = arguments
                    .get("comment")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        "submit_move requires non-empty theater commentary.".to_string()
                    })?
                    .to_string();
                let mut probe = turn.game.clone();
                let info = probe
                    .apply_uci(raw_move)
                    .or_else(|_| probe.apply_san(raw_move))
                    .map_err(|error| format!("'{raw_move}' is not legal: {error}"))?;
                if !turn.candidates.is_empty()
                    && !turn
                        .candidates
                        .iter()
                        .any(|candidate| candidate.uci == info.uci)
                {
                    return Err(format!(
                        "'{}' is outside the Stockfish candidate pool.",
                        info.uci
                    ));
                }
                let candidates = arguments
                    .get("candidates")
                    .cloned()
                    .map(serde_json::from_value::<Vec<AgentCandidate>>)
                    .transpose()
                    .map_err(|error| format!("Invalid candidates array: {error}"))?
                    .unwrap_or_default();
                let submission = Submission::Move(MoveSubmission {
                    mv: info.uci.clone(),
                    comment,
                    eval: arguments
                        .get("eval")
                        .and_then(Value::as_str)
                        .map(String::from),
                    candidates,
                    plan: arguments
                        .get("plan")
                        .and_then(Value::as_str)
                        .map(String::from),
                });
                state.submission = Some(submission);
                Ok(json!({ "uci": info.uci, "san": info.san }))
            }
            "submit_chat" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                if turn.kind != TurnKind::Chat {
                    return Err(format!(
                        "This is a {} turn. Call {} instead.",
                        turn.kind.as_str(),
                        match turn.kind {
                            TurnKind::Move => "submit_move",
                            TurnKind::Draw => "submit_draw",
                            TurnKind::Chat => "submit_chat",
                        }
                    ));
                }
                if state.submission.is_some() {
                    return Err("A response has already been submitted for this turn.".to_string());
                }
                let message = arguments
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "submit_chat requires a non-empty `message`.".to_string())?
                    .chars()
                    .take(4_000)
                    .collect::<String>();
                state.submission = Some(Submission::Chat(ChatSubmission { message }));
                Ok(json!({ "submitted": true }))
            }
            "submit_draw" => {
                let turn = state
                    .turn
                    .as_ref()
                    .ok_or_else(|| "No active Gambiit turn.".to_string())?;
                if turn.kind != TurnKind::Draw {
                    return Err(format!(
                        "This is a {} turn. Call {} instead.",
                        turn.kind.as_str(),
                        match turn.kind {
                            TurnKind::Move => "submit_move",
                            TurnKind::Chat => "submit_chat",
                            TurnKind::Draw => "submit_draw",
                        }
                    ));
                }
                if state.submission.is_some() {
                    return Err("A response has already been submitted for this turn.".to_string());
                }
                let accept = arguments
                    .get("accept")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| "submit_draw requires a boolean `accept`.".to_string())?;
                let comment = arguments
                    .get("comment")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| "submit_draw requires a non-empty `comment`.".to_string())?
                    .chars()
                    .take(800)
                    .collect::<String>();
                state.submission = Some(Submission::Draw(DrawSubmission { accept, comment }));
                Ok(json!({ "accepted": accept }))
            }
            other => Err(format!("Unknown bridge method '{other}'.")),
        }
    }
}

pub fn start() -> Result<Arc<Bridge>, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let token = new_token();
    let state = Arc::new(Mutex::new(BridgeState {
        turn: None,
        submission: None,
    }));
    let bridge = Arc::new(Bridge {
        addr,
        token: token.clone(),
        state: state.clone(),
    });
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let state = state.clone();
            let token = token.clone();
            thread::spawn(move || {
                if let Err(error) = handle_connection(stream, &state, &token) {
                    eprintln!("Gambiit MCP bridge error: {error}");
                }
            });
        }
    });
    Ok(bridge)
}

fn handle_connection(
    stream: TcpStream,
    state: &Arc<Mutex<BridgeState>>,
    token: &str,
) -> Result<(), String> {
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| error.to_string())?);
    let mut writer = stream;
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    let request: WireRequest = serde_json::from_str(line.trim())
        .map_err(|error| format!("Invalid bridge request: {error}"))?;
    let response = if request.token != token {
        json!({ "ok": false, "error": "Invalid Gambiit bridge token." })
    } else {
        // Hold the bridge lock only while reading or writing the turn state.
        let bridge = Bridge {
            addr: "127.0.0.1:0".parse().expect("static address"),
            token: token.to_string(),
            state: state.clone(),
        };
        match bridge.dispatch(&request.method, &request.arguments) {
            Ok(result) => json!({ "ok": true, "result": result }),
            Err(error) => json!({ "ok": false, "error": error }),
        }
    };
    writer
        .write_all(format!("{response}\n").as_bytes())
        .map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn new_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{:x}{:x}", process::id(), nanos)
}

/// Run the stdio MCP server. This is called when the app binary is launched
/// with `--mcp-server` by OpenCode.
pub fn run_stdio() -> Result<(), String> {
    let url = env::var("GAMBIIT_MCP_URL")
        .map_err(|_| "GAMBIIT_MCP_URL is not set for the MCP server.".to_string())?;
    let token = env::var("GAMBIIT_MCP_TOKEN")
        .map_err(|_| "GAMBIIT_MCP_TOKEN is not set for the MCP server.".to_string())?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = request.get("id").cloned();
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        if id.is_none() {
            continue;
        }
        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "gambiit", "version": "0.1.0" }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tool_definitions() }
            }),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let method = match name {
                    "get_state" => Some("state"),
                    "get_legal_moves" => Some("legal_moves"),
                    "get_candidates" => Some("candidates"),
                    "get_clock" => Some("clock"),
                    "get_analysis" => Some("analysis"),
                    "submit_move" => Some("submit"),
                    "submit_chat" => Some("submit_chat"),
                    "submit_draw" => Some("submit_draw"),
                    _ => None,
                };
                let result = match method {
                    Some(method) => call_bridge(&url, &token, method, &arguments),
                    None => Err(format!("Unknown Gambiit tool '{name}'.")),
                };
                match result {
                    Ok(value) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&value).unwrap_or_default()
                            }],
                            "isError": false
                        }
                    }),
                    Err(error) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": error }],
                            "isError": true
                        }
                    }),
                }
            }
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            "resources/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "resources": [] }
            }),
            other => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {other}") }
            }),
        };
        writeln!(out, "{response}").map_err(|error| error.to_string())?;
        out.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn call_bridge(url: &str, token: &str, method: &str, arguments: &Value) -> Result<Value, String> {
    let mut stream = TcpStream::connect(url).map_err(|error| error.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let request = json!({
        "token": token,
        "method": method,
        "arguments": arguments,
    });
    writeln!(stream, "{request}").map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    let response: Value = serde_json::from_str(line.trim()).map_err(|error| error.to_string())?;
    if response.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    } else {
        Err(response
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Gambiit bridge error")
            .to_string())
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "get_state",
            "description": "Read the current Gambiit board state: FEN, turn, status, history and clock.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "get_legal_moves",
            "description": "List every legal move for the side to move in UCI and SAN.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "get_candidates",
            "description": "List the Stockfish candidate moves for this turn. You must play one of these.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "get_clock",
            "description": "Read the remaining clock time for both sides.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "get_analysis",
            "description": "Read the latest post-game Stockfish analysis summary (if one has been run) plus the current position evaluation, best move and candidate list. The summary is unavailable until post-game analysis has been run.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "submit_move",
            "description": "During a move turn only: submit exactly one chess move plus theater commentary. Call this once, then stop.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "move": { "type": "string", "description": "UCI (preferred) or SAN legal candidate move." },
                    "comment": { "type": "string", "description": "Non-empty public theater commentary in the agent language." },
                    "eval": { "type": "string", "description": "Optional claimed evaluation such as +0.3." },
                    "plan": { "type": "string", "description": "Optional public plan; it may be misleading." },
                    "candidates": {
                        "type": "array",
                        "description": "Optional list of candidate moves you considered.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "move": { "type": "string" },
                                "why": { "type": "string" },
                                "why_rejected": { "type": "string" }
                            },
                            "required": ["move"]
                        }
                    }
                },
                "required": ["move", "comment"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "submit_chat",
            "description": "During a chat turn only: send exactly one in-character chat reply to the human. Call this once, then stop.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "The chat reply in the agent language." }
                },
                "required": ["message"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "submit_draw",
            "description": "During a draw-offer turn only: accept or decline the human's draw offer with a short in-character comment. Call this once, then stop.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "accept": { "type": "boolean", "description": "Whether to accept the draw." },
                    "comment": { "type": "string", "description": "Short reply to the human in the agent language." }
                },
                "required": ["accept", "comment"],
                "additionalProperties": false
            }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gambiit_engine::Candidate;

    fn candidate(uci: &str) -> Candidate {
        Candidate {
            uci: uci.to_string(),
            score_cp: Some(20),
            mate_in: None,
            depth: 10,
            multipv: 1,
            pv: Vec::new(),
        }
    }

    #[test]
    fn tool_definitions_expose_chat_draw_and_analysis() {
        let definitions = tool_definitions();
        let names = definitions
            .iter()
            .filter_map(|tool| tool.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        for expected in ["submit_move", "submit_chat", "submit_draw", "get_analysis"] {
            assert!(
                names.contains(&expected),
                "missing MCP tool {expected}: {names:?}"
            );
        }
    }

    #[test]
    fn bridge_serves_state_and_accepts_a_valid_submission() {
        let bridge = start().unwrap();
        let game = Game::new();
        bridge.begin(TurnContext {
            kind: TurnKind::Move,
            legal: game.legal_moves(),
            game,
            candidates: vec![candidate("e2e4"), candidate("d2d4")],
            clock: None,
            analysis: None,
        });
        let url = bridge.url();
        let token = bridge.token();
        let state = call_bridge(&url, token, "state", &json!({})).unwrap();
        assert!(state.get("fen").and_then(Value::as_str).is_some());
        assert_eq!(state.get("turnKind").and_then(Value::as_str), Some("move"));
        let candidates = call_bridge(&url, token, "candidates", &json!({})).unwrap();
        assert_eq!(candidates.as_array().unwrap().len(), 2);

        let submitted = call_bridge(
            &url,
            token,
            "submit",
            &json!({
                "move": "e2e4",
                "comment": "The center is mine.",
                "eval": "+0.2",
                "candidates": [{ "move": "e2e4", "why": "claims the center" }]
            }),
        )
        .unwrap();
        assert_eq!(submitted.get("san").and_then(Value::as_str), Some("e4"));
        match bridge.take_submission().unwrap() {
            Submission::Move(submission) => {
                assert_eq!(submission.mv, "e2e4");
                assert_eq!(submission.comment, "The center is mine.");
                assert_eq!(submission.candidates.len(), 1);
            }
            other => panic!("expected a move submission, got {other:?}"),
        }
        bridge.clear();
    }

    #[test]
    fn bridge_supports_chat_draw_and_analysis() {
        let bridge = start().unwrap();
        let game = Game::new();
        let legal = game.legal_moves();
        bridge.begin(TurnContext {
            kind: TurnKind::Chat,
            game: game.clone(),
            legal: legal.clone(),
            candidates: vec![candidate("e2e4")],
            clock: None,
            analysis: Some(json!({ "agentAccuracy": 80.0, "truthLensFlags": 1 })),
        });

        let analysis = call_bridge(&bridge.url(), bridge.token(), "analysis", &json!({})).unwrap();
        assert_eq!(
            analysis.get("available").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(analysis["summary"]["agentAccuracy"], 80.0);
        assert_eq!(analysis["position"]["best"]["san"], "e4");

        let wrong = call_bridge(
            &bridge.url(),
            bridge.token(),
            "submit",
            &json!({ "move": "e2e4", "comment": "No." }),
        )
        .unwrap_err();
        assert!(wrong.contains("chat turn"));

        call_bridge(
            &bridge.url(),
            bridge.token(),
            "submit_chat",
            &json!({ "message": "Hello from the stage." }),
        )
        .unwrap();
        match bridge.take_submission().unwrap() {
            Submission::Chat(chat) => assert_eq!(chat.message, "Hello from the stage."),
            other => panic!("expected a chat submission, got {other:?}"),
        }

        bridge.begin(TurnContext {
            kind: TurnKind::Draw,
            game: game.clone(),
            legal: game.legal_moves(),
            candidates: vec![candidate("e2e4")],
            clock: None,
            analysis: None,
        });
        let analysis = call_bridge(&bridge.url(), bridge.token(), "analysis", &json!({})).unwrap();
        assert_eq!(
            analysis.get("available").and_then(Value::as_bool),
            Some(false)
        );
        assert!(analysis["summary"].is_null());

        let wrong = call_bridge(
            &bridge.url(),
            bridge.token(),
            "submit_chat",
            &json!({ "message": "No." }),
        )
        .unwrap_err();
        assert!(wrong.contains("draw turn"));

        call_bridge(
            &bridge.url(),
            bridge.token(),
            "submit_draw",
            &json!({ "accept": false, "comment": "I decline." }),
        )
        .unwrap();
        match bridge.take_submission().unwrap() {
            Submission::Draw(draw) => {
                assert!(!draw.accept);
                assert_eq!(draw.comment, "I decline.");
            }
            other => panic!("expected a draw submission, got {other:?}"),
        }
        bridge.clear();
    }

    #[test]
    fn bridge_rejects_moves_outside_the_candidate_pool() {
        let bridge = start().unwrap();
        let game = Game::new();
        bridge.begin(TurnContext {
            kind: TurnKind::Move,
            legal: game.legal_moves(),
            game,
            candidates: vec![candidate("e2e4")],
            clock: None,
            analysis: None,
        });
        let error = call_bridge(
            &bridge.url(),
            bridge.token(),
            "submit",
            &json!({ "move": "d2d4", "comment": "No." }),
        )
        .unwrap_err();
        assert!(error.contains("outside the Stockfish candidate pool"));
        bridge.clear();
    }
}
