//! Minimal Stockfish UCI client used for hybrid move selection and analysis.
//!
//! Stockfish is an external GPL binary. Gambiit detects it but does not bundle
//! it; if it is missing, the game falls back to legal-moves-only play.

use serde::Serialize;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Stockfish was not found. Install it, e.g. `sudo pacman -S stockfish`.")]
    NotFound,
    #[error("failed to start Stockfish: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Stockfish I/O error: {0}")]
    Io(String),
    #[error("Stockfish did not complete the UCI handshake")]
    Handshake,
    #[error("Stockfish search failed: {0}")]
    Search(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub uci: String,
    pub score_cp: Option<i32>,
    pub mate_in: Option<i32>,
    pub depth: u32,
    pub multipv: u32,
    pub pv: Vec<String>,
}

impl Candidate {
    pub fn score_text(&self) -> String {
        if let Some(mate) = self.mate_in {
            format!("#{mate}")
        } else if let Some(cp) = self.score_cp {
            format!("{:+.2}", cp as f64 / 100.0)
        } else {
            "?".to_string()
        }
    }
}

pub struct Stockfish {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    path: PathBuf,
    id: String,
}

impl Stockfish {
    /// Find `stockfish` on PATH, or use `STOCKFISH_BIN` when set.
    pub fn discover() -> Option<PathBuf> {
        if let Some(path) = std::env::var_os("STOCKFISH_BIN") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }

        let path_var = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("stockfish");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn new(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let path = path.as_ref().to_path_buf();
        let mut child = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| EngineError::Io("missing Stockfish stdin".into()))?;
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| EngineError::Io("missing Stockfish stdout".into()))?,
        );

        let mut engine = Self {
            child,
            stdin,
            stdout,
            path,
            id: String::new(),
        };
        engine.id = engine.handshake()?;
        engine.set_option("Threads", "1")?;
        engine.set_option("Hash", "64")?;
        Ok(engine)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn handshake(&mut self) -> Result<String, EngineError> {
        self.send("uci")?;
        let mut id = String::new();
        loop {
            let line = self.read_line()?;
            if let Some(rest) = line.strip_prefix("id name ") {
                id = rest.to_string();
            }
            if line == "uciok" {
                break;
            }
        }
        if id.is_empty() {
            return Err(EngineError::Handshake);
        }
        Ok(id)
    }

    fn send(&mut self, command: &str) -> Result<(), EngineError> {
        writeln!(self.stdin, "{command}").map_err(|e| EngineError::Io(e.to_string()))?;
        self.stdin
            .flush()
            .map_err(|e| EngineError::Io(e.to_string()))
    }

    fn read_line(&mut self) -> Result<String, EngineError> {
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .map_err(|e| EngineError::Io(e.to_string()))?;
        if read == 0 {
            return Err(EngineError::Io("unexpected EOF from Stockfish".into()));
        }
        Ok(line.trim_end().to_string())
    }

    pub fn set_option(&mut self, name: &str, value: &str) -> Result<(), EngineError> {
        self.send(&format!("setoption name {name} value {value}"))
    }

    /// Return the top `multipv` moves at the requested depth, sorted by rank.
    pub fn candidates(
        &mut self,
        fen: &str,
        multipv: usize,
        depth: u32,
    ) -> Result<Vec<Candidate>, EngineError> {
        let multipv = multipv.max(1);
        self.set_option("MultiPV", &multipv.to_string())?;
        self.send(&format!("position fen {fen}"))?;
        self.send(&format!("go depth {depth}"))?;

        let mut best: Vec<Candidate> = Vec::new();
        loop {
            let line = self.read_line()?;
            if line.starts_with("bestmove") {
                break;
            }
            if line.starts_with("info ")
                && line.contains(" pv ")
                && let Some(candidate) = parse_info(&line)
            {
                if let Some(existing) = best
                    .iter_mut()
                    .find(|existing| existing.multipv == candidate.multipv)
                {
                    if candidate.depth >= existing.depth {
                        *existing = candidate;
                    }
                } else {
                    best.push(candidate);
                }
            }
        }

        best.sort_by_key(|candidate| candidate.multipv);
        if best.is_empty() {
            return Err(EngineError::Search("no candidate moves returned".into()));
        }
        Ok(best)
    }

    pub fn best_move(&mut self, fen: &str, depth: u32) -> Result<String, EngineError> {
        let candidates = self.candidates(fen, 1, depth)?;
        candidates
            .into_iter()
            .next()
            .map(|candidate| candidate.uci)
            .ok_or_else(|| EngineError::Search("no best move returned".into()))
    }
}

impl Drop for Stockfish {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        let _ = self.child.wait();
    }
}

fn parse_info(line: &str) -> Option<Candidate> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut depth = 0u32;
    let mut multipv = 1u32;
    let mut score_cp = None;
    let mut mate_in = None;
    let mut pv: Vec<String> = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "depth" => {
                depth = tokens.get(i + 1)?.parse().ok()?;
                i += 2;
            }
            "multipv" => {
                multipv = tokens.get(i + 1)?.parse().ok()?;
                i += 2;
            }
            "score" => match *tokens.get(i + 1)? {
                "cp" => {
                    score_cp = Some(tokens.get(i + 2)?.parse().ok()?);
                    i += 3;
                }
                "mate" => {
                    mate_in = Some(tokens.get(i + 2)?.parse().ok()?);
                    i += 3;
                }
                _ => i += 1,
            },
            "pv" => {
                pv = tokens[i + 1..]
                    .iter()
                    .map(|token| token.to_string())
                    .collect();
                break;
            }
            _ => i += 1,
        }
    }

    if pv.is_empty() {
        return None;
    }

    Some(Candidate {
        uci: pv[0].clone(),
        score_cp,
        mate_in,
        depth,
        multipv,
        pv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stockfish_info_line() {
        let line = "info depth 18 seldepth 24 multipv 2 score cp -34 nodes 123456 nps 900000 hashfull 12 tbhits 0 time 137 pv e7e5 g1f3 b8c6";
        let candidate = parse_info(line).expect("candidate");
        assert_eq!(candidate.uci, "e7e5");
        assert_eq!(candidate.multipv, 2);
        assert_eq!(candidate.depth, 18);
        assert_eq!(candidate.score_cp, Some(-34));
        assert_eq!(candidate.mate_in, None);
        assert_eq!(candidate.pv.len(), 3);
    }

    #[test]
    fn parses_mate_score() {
        let line = "info depth 12 multipv 1 score mate 3 pv d1h5 g8f6 h5f7";
        let candidate = parse_info(line).expect("candidate");
        assert_eq!(candidate.mate_in, Some(3));
        assert_eq!(candidate.score_text(), "#3");
    }
}
