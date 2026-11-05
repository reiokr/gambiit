//! Chess rules and game state for Gambiit.
//!
//! `shakmaty` is the source of truth for legality. This crate adds the
//! game-level concerns Gambiit needs: move records, repetition tracking,
//! resignations and human/agent-facing status text.

use serde::Serialize;
use shakmaty::{
    CastlingMode, Chess, Color, EnPassantMode, KnownOutcome, Move, Outcome, Piece, Position, Role,
    Square, fen::Fen, san::San, uci::UciMove,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("invalid FEN: {0}")]
    Fen(String),
    #[error("invalid position: {0}")]
    Position(String),
    #[error("invalid UCI move: {0}")]
    Uci(String),
    #[error("invalid SAN: {0}")]
    San(String),
    #[error("illegal move: {0}")]
    Illegal(String),
    #[error("game is already over")]
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Ongoing,
    Check,
    Checkmate { winner: Color },
    Stalemate,
    Draw { reason: &'static str },
    Resigned { winner: Color },
    Aborted,
}

impl GameStatus {
    pub fn is_over(self) -> bool {
        !matches!(self, GameStatus::Ongoing | GameStatus::Check)
    }

    pub fn winner(self) -> Option<Color> {
        match self {
            GameStatus::Checkmate { winner } | GameStatus::Resigned { winner } => Some(winner),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MoveInfo {
    pub uci: String,
    pub san: String,
    pub from: String,
    pub to: String,
    pub promotion: Option<String>,
    pub piece: String,
    pub captured: Option<String>,
    pub is_castle: bool,
    pub is_en_passant: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoveRecord {
    pub ply: u32,
    pub san: String,
    pub uci: String,
    pub from: String,
    pub to: String,
    pub fen_after: String,
    pub comment: Option<String>,
    pub eval: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Game {
    initial_fen: String,
    position: Chess,
    history: Vec<MoveRecord>,
    position_keys: Vec<String>,
    human_color: Color,
    manual_status: Option<GameStatus>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        let position = Chess::default();
        let initial_fen = Fen::from_position(&position, EnPassantMode::Legal).to_string();
        let mut game = Self {
            initial_fen,
            position,
            history: Vec::new(),
            position_keys: Vec::new(),
            human_color: Color::White,
            manual_status: None,
        };
        game.position_keys.push(game.repetition_key());
        game
    }

    pub fn from_fen(fen: &str) -> Result<Self, GameError> {
        let parsed = Fen::from_ascii(fen.as_bytes()).map_err(|e| GameError::Fen(e.to_string()))?;
        let position: Chess = parsed
            .into_position(CastlingMode::Standard)
            .map_err(|e| GameError::Position(e.to_string()))?;
        let initial_fen = Fen::from_position(&position, EnPassantMode::Legal).to_string();
        let mut game = Self {
            initial_fen,
            position,
            history: Vec::new(),
            position_keys: Vec::new(),
            human_color: Color::White,
            manual_status: None,
        };
        game.position_keys.push(game.repetition_key());
        Ok(game)
    }

    pub fn initial_fen(&self) -> &str {
        &self.initial_fen
    }

    pub fn fen(&self) -> String {
        Fen::from_position(&self.position, EnPassantMode::Legal).to_string()
    }

    pub fn position(&self) -> &Chess {
        &self.position
    }

    pub fn turn(&self) -> Color {
        self.position.turn()
    }

    pub fn human_color(&self) -> Color {
        self.human_color
    }

    pub fn set_human_color(&mut self, color: Color) {
        self.human_color = color;
    }

    pub fn history(&self) -> &[MoveRecord] {
        &self.history
    }

    pub fn last_move(&self) -> Option<&MoveRecord> {
        self.history.last()
    }

    pub fn legal_moves(&self) -> Vec<MoveInfo> {
        self.position
            .legal_moves()
            .iter()
            .map(|m| self.move_info(*m))
            .collect()
    }

    pub fn legal_uci(&self) -> Vec<String> {
        self.legal_moves().into_iter().map(|m| m.uci).collect()
    }

    pub fn legal_san(&self) -> Vec<String> {
        self.legal_moves().into_iter().map(|m| m.san).collect()
    }

    pub fn apply_uci(&mut self, uci: &str) -> Result<MoveInfo, GameError> {
        if self.status().is_over() {
            return Err(GameError::GameOver);
        }
        let parsed = UciMove::from_ascii(uci.trim().as_bytes())
            .map_err(|e| GameError::Uci(e.to_string()))?;
        let mv = parsed
            .to_move(&self.position)
            .map_err(|e| GameError::Illegal(e.to_string()))?;
        self.apply_move(mv)
    }

    pub fn apply_san(&mut self, san: &str) -> Result<MoveInfo, GameError> {
        if self.status().is_over() {
            return Err(GameError::GameOver);
        }
        let parsed =
            San::from_ascii(san.trim().as_bytes()).map_err(|e| GameError::San(e.to_string()))?;
        let mv = parsed
            .to_move(&self.position)
            .map_err(|e| GameError::San(e.to_string()))?;
        self.apply_move(mv)
    }

    pub fn apply_move(&mut self, mv: Move) -> Result<MoveInfo, GameError> {
        if self.status().is_over() {
            return Err(GameError::GameOver);
        }
        let info = self.move_info(mv);
        self.position = self
            .position
            .clone()
            .play(mv)
            .map_err(|e| GameError::Illegal(e.to_string()))?;
        self.history.push(MoveRecord {
            ply: self.history.len() as u32 + 1,
            san: info.san.clone(),
            uci: info.uci.clone(),
            from: info.from.clone(),
            to: info.to.clone(),
            fen_after: self.fen(),
            comment: None,
            eval: None,
        });
        self.position_keys.push(self.repetition_key());
        Ok(info)
    }

    pub fn annotate_last(&mut self, comment: Option<String>, eval: Option<String>) {
        if let Some(last) = self.history.last_mut() {
            last.comment = comment;
            last.eval = eval;
        }
    }

    pub fn resign(&mut self, winner: Color) {
        self.manual_status = Some(GameStatus::Resigned { winner });
    }

    pub fn abort(&mut self) {
        self.manual_status = Some(GameStatus::Aborted);
    }

    pub fn status(&self) -> GameStatus {
        if let Some(status) = self.manual_status {
            return status;
        }

        if self.position.halfmoves() >= 100 {
            return GameStatus::Draw {
                reason: "50-move rule",
            };
        }

        let key = self.repetition_key();
        let repeats = self
            .position_keys
            .iter()
            .filter(|existing| *existing == &key)
            .count();
        if repeats >= 3 {
            return GameStatus::Draw {
                reason: "threefold repetition",
            };
        }

        match self.position.outcome() {
            Outcome::Known(KnownOutcome::Decisive { winner }) => GameStatus::Checkmate { winner },
            Outcome::Known(KnownOutcome::Draw) => {
                if self.position.is_stalemate() {
                    GameStatus::Stalemate
                } else {
                    GameStatus::Draw {
                        reason: "insufficient material",
                    }
                }
            }
            Outcome::Unknown => {
                if self.position.is_check() {
                    GameStatus::Check
                } else {
                    GameStatus::Ongoing
                }
            }
        }
    }

    pub fn status_text(&self) -> String {
        let turn = color_name(self.turn());
        match self.status() {
            GameStatus::Ongoing => format!("{turn} to move."),
            GameStatus::Check => format!("{turn} is in check."),
            GameStatus::Checkmate { winner } => {
                format!("Checkmate — {} wins.", color_name(winner))
            }
            GameStatus::Stalemate => "Stalemate — draw.".to_string(),
            GameStatus::Draw { reason } => format!("Draw — {reason}."),
            GameStatus::Resigned { winner } => {
                format!("{} wins — opponent resigned.", color_name(winner))
            }
            GameStatus::Aborted => "Game aborted.".to_string(),
        }
    }

    pub fn is_check(&self) -> bool {
        self.position.is_check()
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.position.board().piece_at(square)
    }

    pub fn repetition_key(&self) -> String {
        let fen = Fen::from_position(&self.position, EnPassantMode::Legal).to_string();
        // Board, side to move, castling rights, en passant target. Ignore the
        // halfmove/fullmove counters so repeated positions compare correctly.
        fen.split(' ').take(4).collect::<Vec<_>>().join(" ")
    }

    pub fn pgn_moves(&self) -> String {
        let mut out = String::new();
        for (i, record) in self.history.iter().enumerate() {
            if i % 2 == 0 {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&format!("{}. ", i / 2 + 1));
            }
            out.push_str(&record.san);
            out.push(' ');
        }
        out.trim_end().to_string()
    }

    pub fn perft(&self, depth: u32) -> u64 {
        perft(&self.position, depth)
    }

    fn move_info(&self, mv: Move) -> MoveInfo {
        let uci = mv.to_uci(self.position.castles().mode()).to_string();
        let san = shakmaty::san::SanPlus::from_move(self.position.clone(), mv).to_string();
        let from = uci.get(0..2).unwrap_or("??").to_string();
        let to = uci.get(2..4).unwrap_or("??").to_string();
        let promotion = uci.get(4..).filter(|p| !p.is_empty()).map(String::from);
        MoveInfo {
            uci,
            san,
            from,
            to,
            promotion,
            piece: role_letter(mv.role()).to_string(),
            captured: mv.capture().map(|role| role_letter(role).to_string()),
            is_castle: mv.is_castle(),
            is_en_passant: mv.is_en_passant(),
        }
    }
}

pub fn perft(position: &Chess, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut nodes = 0;
    for mv in position.legal_moves() {
        let mut next = position.clone();
        next.play_unchecked(mv);
        nodes += perft(&next, depth - 1);
    }
    nodes
}

pub fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "White",
        Color::Black => "Black",
    }
}

pub fn role_letter(role: Role) -> char {
    match role {
        Role::Pawn => 'P',
        Role::Knight => 'N',
        Role::Bishop => 'B',
        Role::Rook => 'R',
        Role::Queen => 'Q',
        Role::King => 'K',
    }
}

pub fn square_index(file: u32, rank: u32) -> u32 {
    rank * 8 + file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perft_start_position() {
        let game = Game::new();
        assert_eq!(game.perft(0), 1);
        assert_eq!(game.perft(1), 20);
        assert_eq!(game.perft(2), 400);
        assert_eq!(game.perft(3), 8_902);
    }

    #[test]
    fn applies_uci_and_san() {
        let mut game = Game::new();
        let info = game.apply_uci("e2e4").unwrap();
        assert_eq!(info.san, "e4");
        assert_eq!(game.turn(), Color::Black);
        game.apply_san("e7e5").unwrap();
        assert_eq!(game.turn(), Color::White);
        assert_eq!(game.history().len(), 2);
        assert_eq!(game.pgn_moves(), "1. e4 e5");
    }

    #[test]
    fn rejects_illegal_move() {
        let mut game = Game::new();
        assert!(game.apply_uci("e2e5").is_err());
        assert_eq!(game.history().len(), 0);
    }

    #[test]
    fn detects_checkmate() {
        let game = Game::from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        assert_eq!(
            game.status(),
            GameStatus::Checkmate {
                winner: Color::White
            }
        );
    }

    #[test]
    fn detects_stalemate() {
        let game = Game::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        assert_eq!(game.status(), GameStatus::Stalemate);
    }

    #[test]
    fn detects_threefold_repetition() {
        let mut game = Game::new();
        for san in ["Nf3", "Nf6", "Ng1", "Ng8", "Nf3", "Nf6", "Ng1", "Ng8"] {
            game.apply_san(san).unwrap();
        }
        assert_eq!(
            game.status(),
            GameStatus::Draw {
                reason: "threefold repetition"
            }
        );
    }

    #[test]
    fn castle_uci_is_king_move() {
        let mut game = Game::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let info = game.apply_uci("e1g1").unwrap();
        assert_eq!(info.san, "O-O");
        assert!(info.is_castle);
        assert_eq!(info.from, "e1");
        assert_eq!(info.to, "g1");
    }
}
