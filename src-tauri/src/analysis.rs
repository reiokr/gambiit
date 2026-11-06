//! Post-game Stockfish analysis and Truth Lens comparison.

use gambiit_core::{Game, GameStatus};
use gambiit_engine::Stockfish;
use serde::{Deserialize, Serialize};
use shakmaty::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisPly {
    pub ply: u32,
    pub san: String,
    pub uci: String,
    pub color: String,
    pub fen_before: String,
    pub fen_after: String,
    pub eval_before_cp: i32,
    pub eval_after_cp: i32,
    pub best_uci: Option<String>,
    pub best_san: Option<String>,
    pub loss_cp: i32,
    pub classification: String,
    pub claimed_eval: Option<String>,
    pub claim_gap_cp: Option<i32>,
    pub claim_mismatch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisReport {
    pub depth: u32,
    pub initial_eval_cp: i32,
    pub positions: Vec<AnalysisPly>,
    pub white_accuracy: f64,
    pub black_accuracy: f64,
    pub white_blunders: u32,
    pub black_blunders: u32,
    pub white_mistakes: u32,
    pub black_mistakes: u32,
    pub white_inaccuracies: u32,
    pub black_inaccuracies: u32,
    pub truth_lens_flags: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisSummary {
    pub depth: u32,
    pub analyzed_plies: u32,
    pub agent_accuracy: f64,
    pub agent_moves: u32,
    pub agent_blunders: u32,
    pub agent_mistakes: u32,
    pub agent_inaccuracies: u32,
    pub truth_lens_flags: u32,
}

impl AnalysisSummary {
    pub fn from_report(report: &AnalysisReport, agent_is_white: bool) -> Self {
        let agent_color = if agent_is_white { "white" } else { "black" };
        let positions = report
            .positions
            .iter()
            .filter(|position| position.color == agent_color)
            .collect::<Vec<_>>();
        let mut summary = Self {
            depth: report.depth,
            analyzed_plies: report.positions.len() as u32,
            agent_accuracy: if agent_is_white {
                report.white_accuracy
            } else {
                report.black_accuracy
            },
            agent_moves: positions.len() as u32,
            agent_blunders: if agent_is_white {
                report.white_blunders
            } else {
                report.black_blunders
            },
            agent_mistakes: if agent_is_white {
                report.white_mistakes
            } else {
                report.black_mistakes
            },
            agent_inaccuracies: if agent_is_white {
                report.white_inaccuracies
            } else {
                report.black_inaccuracies
            },
            truth_lens_flags: positions
                .iter()
                .filter(|position| position.claim_mismatch)
                .count() as u32,
        };
        if summary.agent_moves == 0 {
            summary.agent_accuracy = 0.0;
        }
        summary
    }
}

#[derive(Debug, Clone)]
struct EngineEval {
    cp_white: i32,
    best_uci: Option<String>,
    best_san: Option<String>,
}

pub(crate) fn mate_cp(mate_in: i32, turn: Color) -> i32 {
    let magnitude = 10_000;
    if mate_in > 0 {
        if turn == Color::White {
            magnitude
        } else {
            -magnitude
        }
    } else if turn == Color::White {
        -magnitude
    } else {
        magnitude
    }
}

fn eval_position(engine: &mut Stockfish, fen: &str, depth: u32) -> Result<EngineEval, String> {
    let position = Game::from_fen(fen).map_err(|error| error.to_string())?;
    let turn = position.turn();

    match position.status() {
        GameStatus::Checkmate { winner } => {
            return Ok(EngineEval {
                cp_white: if winner == Color::White {
                    10_000
                } else {
                    -10_000
                },
                best_uci: None,
                best_san: None,
            });
        }
        status if status.is_over() => {
            return Ok(EngineEval {
                cp_white: 0,
                best_uci: None,
                best_san: None,
            });
        }
        _ => {}
    }

    let candidates = engine
        .candidates(fen, 1, depth)
        .map_err(|error| error.to_string())?;
    let candidate = candidates.first();
    let cp_white = candidate
        .and_then(|candidate| {
            if let Some(mate) = candidate.mate_in {
                Some(mate_cp(mate, turn))
            } else {
                candidate
                    .score_cp
                    .map(|cp| if turn == Color::White { cp } else { -cp })
            }
        })
        .unwrap_or(0);
    let best_uci = candidate.map(|candidate| candidate.uci.clone());
    let best_san = best_uci.as_ref().and_then(|uci| {
        let mut game = Game::from_fen(fen).ok()?;
        game.apply_uci(uci).ok().map(|info| info.san)
    });

    Ok(EngineEval {
        cp_white,
        best_uci,
        best_san,
    })
}

fn win_probability(cp: i32) -> f64 {
    1.0 / (1.0 + 10f64.powf(-(cp.clamp(-2_000, 2_000) as f64) / 400.0))
}

pub(crate) fn eval_white_cp(engine: &mut Stockfish, fen: &str, depth: u32) -> Result<i32, String> {
    eval_position(engine, fen, depth).map(|eval| eval.cp_white)
}

pub(crate) fn classify(played: &str, best: Option<&str>, loss_cp: i32) -> &'static str {
    if loss_cp >= 300 {
        "blunder"
    } else if loss_cp >= 150 {
        "mistake"
    } else if loss_cp >= 50 {
        "inaccuracy"
    } else if best.is_some_and(|best| best == played) {
        "best"
    } else {
        "good"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Claim {
    Numeric(i32),
    Mate(i32),
    Winning,
    Losing,
    Equal,
    Better,
    Worse,
}

fn parse_mate_value(text: &str) -> Option<i32> {
    text.trim()
        .trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':'))
        .parse::<i32>()
        .ok()
}

fn claim_from_text(text: &str) -> Option<Claim> {
    let normalized = text.trim().replace('−', "-").to_ascii_lowercase();
    let normalized = normalized
        .trim()
        .trim_matches(|c: char| matches!(c, '.' | ',' | '!' | '?' | ';' | ':'))
        .trim();
    if normalized.is_empty() {
        return None;
    }

    if let Some(rest) = normalized.strip_prefix('#') {
        return parse_mate_value(rest).map(Claim::Mate);
    }
    if let Some(rest) = normalized.strip_prefix('m')
        && let Some(value) = parse_mate_value(rest)
    {
        return Some(Claim::Mate(value));
    }
    for marker in ["mate in ", "matt in ", "matt "] {
        if let Some(index) = normalized.find(marker) {
            let rest = &normalized[index + marker.len()..];
            if let Some(value) = rest.split_whitespace().next().and_then(parse_mate_value) {
                return Some(Claim::Mate(value));
            }
        }
    }

    let tokens = normalized.split_whitespace().collect::<Vec<_>>();
    let mate_word = tokens.iter().any(|token| {
        let token = token
            .trim_matches(|c: char| matches!(c, ',' | '.' | '!' | '?' | ';' | ':' | '(' | ')'));
        matches!(token, "mate" | "matt" | "checkmate")
    });
    if mate_word {
        return Some(Claim::Mate(1));
    }
    for token in &tokens {
        let raw = token.trim_matches(|c: char| matches!(c, ',' | ';' | ':' | '(' | ')'));
        let cleaned = raw.replace(',', ".");
        let cleaned = cleaned.trim_matches('.');
        if let Ok(value) = cleaned.parse::<f64>() {
            let bare_number = tokens.len() == 1 && value.abs() <= 30.0;
            if raw.starts_with('+') || raw.starts_with('-') || cleaned.contains('.') || bare_number
            {
                return Some(Claim::Numeric((value * 100.0).round() as i32));
            }
        }
    }

    for (index, token) in tokens.iter().enumerate() {
        if index > 0 && matches!(tokens[index - 1], "not" | "mitte" | "pole") {
            continue;
        }
        let token = token.trim_matches(|c: char| matches!(c, ',' | '.' | '+' | ';' | ':'));
        match token {
            "winning" | "win" | "won" | "võidan" | "võit" | "võitmas" | "võidab" => {
                return Some(Claim::Winning);
            }
            "losing" | "lost" | "loss" | "kaotan" | "kaotus" | "kaotamas" | "kaotab" => {
                return Some(Claim::Losing);
            }
            "equal" | "draw" | "drawish" | "level" | "viik" | "viigis" | "võrdne" | "tasakaal" => {
                return Some(Claim::Equal);
            }
            "better" | "advantage" | "eelis" | "parem" | "paremal" => {
                return Some(Claim::Better);
            }
            "worse" | "disadvantage" | "puudujääk" | "halvem" | "halvemal" => {
                return Some(Claim::Worse);
            }
            _ => {}
        }
    }
    None
}

fn claim_gap(claim: Claim, actual_cp: i32) -> i32 {
    match claim {
        Claim::Numeric(value) => (value - actual_cp).abs(),
        Claim::Mate(direction) => {
            if actual_cp.abs() >= 9_000 && actual_cp.signum() == direction.signum() {
                0
            } else {
                10_000
            }
        }
        Claim::Winning => (200 - actual_cp).max(0),
        Claim::Losing => (actual_cp + 200).max(0),
        Claim::Equal => (actual_cp.abs() - 100).max(0),
        Claim::Better => (50 - actual_cp).max(0),
        Claim::Worse => (actual_cp + 50).max(0),
    }
}

fn compare_claim(text: &str, actual_cp: i32) -> Option<(i32, bool)> {
    let claim = claim_from_text(text)?;
    let gap = claim_gap(claim, actual_cp);
    Some((gap, gap >= 150))
}

pub fn analyze(game: &Game, depth: u32) -> Result<AnalysisReport, String> {
    let depth = depth.clamp(6, 22);
    let path = Stockfish::discover()
        .ok_or_else(|| "Stockfish was not found. Install it to use game analysis.".to_string())?;
    let mut engine = Stockfish::new(path).map_err(|error| error.to_string())?;
    engine.set_skill(20).map_err(|error| error.to_string())?;

    let history = game.history();
    let mut fens = Vec::with_capacity(history.len() + 1);
    fens.push(game.initial_fen().to_string());
    fens.extend(history.iter().map(|record| record.fen_after.clone()));

    let mut evals = Vec::with_capacity(fens.len());
    for fen in &fens {
        evals.push(eval_position(&mut engine, fen, depth)?);
    }

    let mut positions = Vec::with_capacity(history.len());
    let mut white_accuracy = 0.0;
    let mut black_accuracy = 0.0;
    let mut white_moves = 0u32;
    let mut black_moves = 0u32;
    let mut white_blunders = 0u32;
    let mut black_blunders = 0u32;
    let mut white_mistakes = 0u32;
    let mut black_mistakes = 0u32;
    let mut white_inaccuracies = 0u32;
    let mut black_inaccuracies = 0u32;
    let mut truth_lens_flags = 0u32;

    for (index, record) in history.iter().enumerate() {
        let before = &evals[index];
        let after = &evals[index + 1];
        let white_move = record.color.eq_ignore_ascii_case("white");
        let before_mover = if white_move {
            before.cp_white
        } else {
            -before.cp_white
        };
        let after_mover = if white_move {
            after.cp_white
        } else {
            -after.cp_white
        };
        let loss_cp = (before_mover - after_mover).max(0);
        let accuracy =
            (1.0 - (win_probability(before_mover) - win_probability(after_mover)).max(0.0)) * 100.0;
        if white_move {
            white_accuracy += accuracy;
            white_moves += 1;
        } else {
            black_accuracy += accuracy;
            black_moves += 1;
        }

        let classification = classify(&record.uci, before.best_uci.as_deref(), loss_cp);
        match (white_move, classification) {
            (true, "blunder") => white_blunders += 1,
            (false, "blunder") => black_blunders += 1,
            (true, "mistake") => white_mistakes += 1,
            (false, "mistake") => black_mistakes += 1,
            (true, "inaccuracy") => white_inaccuracies += 1,
            (false, "inaccuracy") => black_inaccuracies += 1,
            _ => {}
        }

        let claimed_eval = record.eval.clone();
        let (claim_gap_cp, claim_mismatch) = claimed_eval
            .as_deref()
            .and_then(|text| compare_claim(text, after_mover))
            .map(|(gap, mismatch)| (Some(gap), mismatch))
            .unwrap_or((None, false));
        if claim_mismatch {
            truth_lens_flags += 1;
        }

        positions.push(AnalysisPly {
            ply: record.ply,
            san: record.san.clone(),
            uci: record.uci.clone(),
            color: record.color.clone(),
            fen_before: fens[index].clone(),
            fen_after: fens[index + 1].clone(),
            eval_before_cp: before.cp_white,
            eval_after_cp: after.cp_white,
            best_uci: before.best_uci.clone(),
            best_san: before.best_san.clone(),
            loss_cp,
            classification: classification.to_string(),
            claimed_eval,
            claim_gap_cp,
            claim_mismatch,
        });
    }

    Ok(AnalysisReport {
        depth,
        initial_eval_cp: evals.first().map(|eval| eval.cp_white).unwrap_or(0),
        positions,
        white_accuracy: if white_moves > 0 {
            white_accuracy / white_moves as f64
        } else {
            0.0
        },
        black_accuracy: if black_moves > 0 {
            black_accuracy / black_moves as f64
        } else {
            0.0
        },
        white_blunders,
        black_blunders,
        white_mistakes,
        black_mistakes,
        white_inaccuracies,
        black_inaccuracies,
        truth_lens_flags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numeric_and_mate_claims() {
        assert_eq!(claim_from_text("+0.35"), Some(Claim::Numeric(35)));
        assert_eq!(claim_from_text("-1.2"), Some(Claim::Numeric(-120)));
        assert_eq!(claim_from_text("−0,5"), Some(Claim::Numeric(-50)));
        assert_eq!(claim_from_text("#3"), Some(Claim::Mate(3)));
        assert_eq!(claim_from_text("mate in 3"), Some(Claim::Mate(3)));
        assert_eq!(claim_from_text("M2"), Some(Claim::Mate(2)));
        assert_eq!(claim_from_text("matt 4"), Some(Claim::Mate(4)));
        assert_eq!(claim_from_text("checkmate"), Some(Claim::Mate(1)));
        assert_eq!(claim_from_text("estimate"), None);
        assert_eq!(claim_from_text("+1"), Some(Claim::Numeric(100)));
        assert_eq!(claim_from_text("1"), Some(Claim::Numeric(100)));
        assert_eq!(claim_from_text("nonsense"), None);
    }

    #[test]
    fn parses_qualitative_claims() {
        assert_eq!(claim_from_text("winning"), Some(Claim::Winning));
        assert_eq!(claim_from_text("I am winning"), Some(Claim::Winning));
        assert_eq!(claim_from_text("losing"), Some(Claim::Losing));
        assert_eq!(claim_from_text("equal"), Some(Claim::Equal));
        assert_eq!(claim_from_text("drawish"), Some(Claim::Equal));
        assert_eq!(claim_from_text("slightly better"), Some(Claim::Better));
        assert_eq!(claim_from_text("3 pawns better"), Some(Claim::Better));
        assert_eq!(claim_from_text("worse"), Some(Claim::Worse));
        assert_eq!(claim_from_text("võidan"), Some(Claim::Winning));
        assert_eq!(claim_from_text("viik"), Some(Claim::Equal));
        assert_eq!(claim_from_text("not winning"), None);
        assert_eq!(claim_from_text("pole parem"), None);
    }

    #[test]
    fn claim_gaps_respect_expected_ranges() {
        assert_eq!(claim_gap(Claim::Winning, 300), 0);
        assert_eq!(claim_gap(Claim::Winning, 0), 200);
        assert_eq!(claim_gap(Claim::Losing, -300), 0);
        assert_eq!(claim_gap(Claim::Losing, 0), 200);
        assert_eq!(claim_gap(Claim::Equal, 50), 0);
        assert_eq!(claim_gap(Claim::Equal, 300), 200);
        assert_eq!(claim_gap(Claim::Better, 0), 50);
        assert_eq!(claim_gap(Claim::Worse, 0), 50);
        assert_eq!(claim_gap(Claim::Mate(3), 10_000), 0);
        assert_eq!(claim_gap(Claim::Mate(3), -10_000), 10_000);
        assert_eq!(claim_gap(Claim::Numeric(50), -100), 150);
    }

    #[test]
    fn compare_claim_flags_only_large_gaps() {
        assert_eq!(compare_claim("winning", -300), Some((500, true)));
        assert_eq!(compare_claim("equal", 20), Some((0, false)));
        assert_eq!(compare_claim("better", 100), Some((0, false)));
        assert_eq!(compare_claim("mate in 2", 10_000), Some((0, false)));
        assert_eq!(compare_claim("mate in 2", -10_000), Some((10_000, true)));
        assert_eq!(compare_claim("nonsense", 0), None);
    }

    #[test]
    fn classifies_moves_by_centipawn_loss() {
        assert_eq!(classify("e2e4", Some("e2e4"), 0), "best");
        assert_eq!(classify("e2e4", Some("d2d4"), 0), "good");
        assert_eq!(classify("e2e4", Some("d2d4"), 60), "inaccuracy");
        assert_eq!(classify("e2e4", Some("d2d4"), 180), "mistake");
        assert_eq!(classify("e2e4", Some("d2d4"), 420), "blunder");
    }

    #[test]
    fn win_probability_is_symmetric() {
        let equal = win_probability(0);
        assert!((equal - 0.5).abs() < 1e-9);
        assert!((win_probability(200) + win_probability(-200) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn mate_scores_use_white_perspective() {
        assert_eq!(mate_cp(2, Color::White), 10_000);
        assert_eq!(mate_cp(-2, Color::White), -10_000);
        assert_eq!(mate_cp(2, Color::Black), -10_000);
        assert_eq!(mate_cp(-2, Color::Black), 10_000);
    }

    #[test]
    #[ignore = "requires Stockfish"]
    fn analyzes_a_short_game_with_stockfish() {
        let mut game = Game::new();
        for mv in ["e4", "e5", "Nf3", "Nc6", "Bb5", "a6"] {
            game.apply_san(mv).unwrap();
        }
        game.annotate_last(Some("A bold claim.".to_string()), Some("+9.9".to_string()));
        let report = analyze(&game, 10).unwrap();
        assert_eq!(report.positions.len(), 6);
        assert_eq!(report.depth, 10);
        assert!(report.white_accuracy > 0.0);
        assert!(report.black_accuracy > 0.0);
        assert!(report.positions.iter().all(|ply| {
            ["best", "good", "inaccuracy", "mistake", "blunder"]
                .contains(&ply.classification.as_str())
        }));
        let last = report.positions.last().unwrap();
        assert!(last.claim_mismatch);
        assert_eq!(report.truth_lens_flags, 1);
    }

    #[test]
    #[ignore = "requires Stockfish"]
    fn flags_qualitative_bluff() {
        let mut game = Game::new();
        for mv in ["f3", "e5", "g4"] {
            game.apply_san(mv).unwrap();
        }
        game.annotate_last(
            Some("I am completely winning.".to_string()),
            Some("winning".to_string()),
        );
        game.apply_san("Qh4#").unwrap();
        let report = analyze(&game, 10).unwrap();
        let g4 = report
            .positions
            .iter()
            .find(|position| position.san == "g4")
            .unwrap();
        assert!(g4.claim_mismatch);
        assert!(g4.claim_gap_cp.unwrap() >= 150);
        assert_eq!(report.truth_lens_flags, 1);
    }

    #[test]
    #[ignore = "requires Stockfish"]
    fn detects_fools_mate_and_blunders() {
        let mut game = Game::new();
        for mv in ["f3", "e5", "g4", "Qh4#"] {
            game.apply_san(mv).unwrap();
        }
        let report = analyze(&game, 10).unwrap();
        let last = report.positions.last().unwrap();
        assert_eq!(last.san, "Qh4#");
        assert_eq!(last.eval_after_cp, -10_000);
        assert_eq!(last.classification, "best");
        let g4 = report
            .positions
            .iter()
            .find(|position| position.san == "g4")
            .unwrap();
        assert_eq!(g4.classification, "blunder");
        assert!(report.black_accuracy > report.white_accuracy);
    }
}
