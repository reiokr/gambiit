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

pub struct TurnContext {
    pub game: Game,
    pub legal: Vec<MoveInfo>,
    pub candidates: Vec<Candidate>,
    pub clock: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct Submission {
    pub mv: String,
    pub comment: String,
    pub eval: Option<String>,
    pub candidates: Vec<AgentCandidate>,
    pub plan: Option<String>,
}

struct BridgeState {
    turn: Option<TurnContext>,
    submission: Option<Submission>,
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
                let candidates: Vec<Value> = turn
                    .candidates
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
                    .collect();
                Ok(Value::Array(candidates))
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
                if state.submission.is_some() {
                    return Err("A move has already been submitted for this turn.".to_string());
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
                let submission = Submission {
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
                };
                state.submission = Some(submission);
                Ok(json!({ "uci": info.uci, "san": info.san }))
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
                    "submit_move" => Some("submit"),
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
            "name": "submit_move",
            "description": "Submit exactly one chess move plus theater commentary for this turn. Call this once, then stop.",
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
    fn bridge_serves_state_and_accepts_a_valid_submission() {
        let bridge = start().unwrap();
        let game = Game::new();
        bridge.begin(TurnContext {
            legal: game.legal_moves(),
            game,
            candidates: vec![candidate("e2e4"), candidate("d2d4")],
            clock: None,
        });
        let url = bridge.url();
        let token = bridge.token();
        let state = call_bridge(&url, token, "state", &json!({})).unwrap();
        assert!(state.get("fen").and_then(Value::as_str).is_some());
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
        let submission = bridge.take_submission().unwrap();
        assert_eq!(submission.mv, "e2e4");
        assert_eq!(submission.comment, "The center is mine.");
        assert_eq!(submission.candidates.len(), 1);
        bridge.clear();
    }

    #[test]
    fn bridge_rejects_moves_outside_the_candidate_pool() {
        let bridge = start().unwrap();
        let game = Game::new();
        bridge.begin(TurnContext {
            legal: game.legal_moves(),
            game,
            candidates: vec![candidate("e2e4")],
            clock: None,
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
