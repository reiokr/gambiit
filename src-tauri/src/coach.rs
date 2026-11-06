//! Honest in-game coach prompts.
//!
//! The coach is deliberately separate from the opponent persona. It receives
//! Stockfish facts and a strict honest-coach instruction; it never bluffs and
//! never changes the board.

use gambiit_core::{Game, color_key, color_name};
use gambiit_engine::{Candidate, Stockfish};
use shakmaty::Color;

use crate::analysis;

/// Coach searches a little deeper than casual play but stays fast enough for
/// an on-demand hint.
pub const DEPTH: u32 = 14;

pub fn format_eval(cp: i32) -> String {
    if cp >= 9_900 {
        "+M".to_string()
    } else if cp <= -9_900 {
        "-M".to_string()
    } else {
        format!("{:+.2}", cp as f64 / 100.0)
    }
}

pub fn candidate_white_cp(candidate: &Candidate, turn: Color) -> Option<i32> {
    if let Some(mate) = candidate.mate_in {
        Some(analysis::mate_cp(mate, turn))
    } else {
        candidate
            .score_cp
            .map(|cp| if turn == Color::White { cp } else { -cp })
    }
}

fn san_for(game: &Game, uci: &str) -> String {
    game.legal_moves()
        .iter()
        .find(|mv| mv.uci == uci)
        .map(|mv| mv.san.clone())
        .unwrap_or_else(|| uci.to_string())
}

fn history_tail(game: &Game, max_plies: usize) -> String {
    let history = game.history();
    let start = history.len().saturating_sub(max_plies);
    let text = history[start..]
        .iter()
        .map(|record| record.san.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        "(no moves yet)".to_string()
    } else {
        text
    }
}

fn honest_rules(language: &str) -> String {
    format!(
        "Honest coach only: no theater, no persona, no bluff. Do not invent board facts, rules or variations. Positive evaluation means better for the human; negative means worse. Reply in {language} as plain text, no JSON, no hidden chain-of-thought."
    )
}

pub fn explain_prompt(game: &Game, human_color: Color, eval_text: &str, language: &str) -> String {
    let human_side = color_name(human_color);
    format!(
        r#"You are an honest chess coach helping the human player during a casual game.
You coach {human_side}. The opponent is a separate AI; do not speak as the opponent.
The board and Stockfish are the truth.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}
Engine evaluation for {human_side}: {eval}

Give a concise position report in {language}, 4-6 sentences:
- material balance and king safety;
- pawn structure, space and open lines;
- the opponent's main threat or plan;
- the human's best practical plan.
Do not recommend a move for the human or name a candidate destination. This is a position report, not a hint. Do not mention the engine's candidate list.

{rules}
"#,
        human_side = human_side,
        fen = game.fen(),
        history = history_tail(game, 30),
        status = game.status_text(),
        eval = eval_text,
        language = language,
        rules = honest_rules(language),
    )
}

pub fn hint_prompt(
    game: &Game,
    human_color: Color,
    level: u8,
    candidates: &[Candidate],
    eval_text: &str,
    language: &str,
) -> String {
    let human_side = color_name(human_color);
    let level = level.clamp(1, 3);
    let level_task = match level {
        1 => "LEVEL 1 — IDEA. Give one thematic idea only: which part of the board or which piece type deserves attention, and why. Do not name any move, candidate, piece destination or engine choice. 1-2 sentences.".to_string(),
        2 => {
            let mut list = candidates.iter().take(3).collect::<Vec<_>>();
            if list.len() > 1 {
                let best = list.remove(0);
                let position = (game.history().len() % list.len()) + 1;
                list.insert(position, best);
            }
            let names = list
                .iter()
                .map(|candidate| san_for(game, &candidate.uci))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "LEVEL 2 — CANDIDATES. These legal engine candidates are deliberately listed in non-best-first order and without scores: {names}. Name 2-3 of them with one short reason each. Do not say which one Stockfish prefers. 2-3 sentences."
            )
        }
        _ => {
            let best = candidates.first();
            let (san, uci) = best
                .map(|candidate| (san_for(game, &candidate.uci), candidate.uci.clone()))
                .unwrap_or_else(|| ("?".to_string(), "?".to_string()));
            format!(
                "LEVEL 3 — MOVE. The engine's preferred move is {san} ({uci}). Reveal it clearly and explain the concrete idea in 2-3 sentences. Mention what the human should watch for after it."
            )
        }
    };

    format!(
        r#"You are an honest chess coach helping the human player during a casual game.
You coach {human_side}. The opponent is a separate AI; do not speak as the opponent.
The board and Stockfish are the truth.

FEN: {fen}
Move history (SAN): {history}
Game status: {status}
Engine evaluation for {human_side}: {eval}

This is hint level {level} of 3.
{level_task}

{rules}
"#,
        human_side = human_side,
        fen = game.fen(),
        history = history_tail(game, 30),
        status = game.status_text(),
        eval = eval_text,
        level = level,
        level_task = level_task,
        rules = honest_rules(language),
    )
}

#[derive(Debug, Clone)]
pub struct ReviewFacts {
    pub played_san: String,
    pub played_uci: String,
    pub best_san: String,
    pub best_uci: String,
    pub before_fen: String,
    pub after_fen: String,
    pub eval_before: String,
    pub eval_after: String,
    pub loss_cp: i32,
    pub classification: String,
    pub ply: u32,
}

/// Build the factual review context for the human player's most recent move.
/// The engine is searched on the position before that move, then the played
/// move is applied and evaluated so the coach receives a real centipawn loss.
pub fn review_facts(
    engine: &mut Stockfish,
    game: &Game,
    human_color: Color,
) -> Result<ReviewFacts, String> {
    let human_key = color_key(human_color);
    let record = game
        .history()
        .iter()
        .rev()
        .find(|record| record.color == human_key)
        .cloned()
        .ok_or_else(|| "There is no human move to review yet.".to_string())?;

    let mut before = game.clone();
    let target_ply = record.ply;
    while before
        .history()
        .last()
        .is_some_and(|last| last.ply >= target_ply)
    {
        before.undo_last();
    }

    let before_fen = before.fen();
    let candidates = engine
        .candidates(&before_fen, 1, DEPTH)
        .map_err(|error| error.to_string())?;
    let best = candidates
        .first()
        .ok_or_else(|| "Stockfish found no move to review.".to_string())?;
    let best_uci = best.uci.clone();
    let best_san = san_for(&before, &best_uci);
    let eval_before_white = candidate_white_cp(best, before.turn())
        .ok_or_else(|| "Stockfish did not return an evaluation.".to_string())?;

    let mut after = before.clone();
    after
        .apply_uci(&record.uci)
        .map_err(|error| error.to_string())?;
    let after_fen = after.fen();
    let eval_after_white = analysis::eval_white_cp(engine, &after_fen, DEPTH)?;

    let (eval_before, eval_after) = if human_color == Color::White {
        (eval_before_white, eval_after_white)
    } else {
        (-eval_before_white, -eval_after_white)
    };
    let mut loss_cp = (eval_before - eval_after).max(0);
    // A move that matches the engine's top choice has no choice-loss, even if
    // the engine's score drifts a little between two searches.
    if record.uci == best_uci {
        loss_cp = 0;
    }
    let classification = analysis::classify(&record.uci, Some(&best_uci), loss_cp).to_string();

    Ok(ReviewFacts {
        played_san: record.san,
        played_uci: record.uci,
        best_san,
        best_uci,
        before_fen,
        after_fen,
        eval_before: format_eval(eval_before),
        eval_after: format_eval(eval_after),
        loss_cp,
        classification,
        ply: record.ply,
    })
}

pub fn review_prompt(facts: &ReviewFacts, language: &str) -> String {
    format!(
        r#"You are an honest chess coach reviewing the human player's last move.
The board and Stockfish are the truth.

The human played {played_san} ({played_uci}) on ply {ply}.
Position before the move: {before_fen}
Position after the move: {after_fen}
Engine's preferred move: {best_san} ({best_uci})
Evaluation before the move, for the human: {eval_before}
Evaluation after the move, for the human: {eval_after}
Centipawn loss: {loss_cp} cp ({loss:.2} pawns)
Engine classification: {classification}

Explain in {language}, 3-5 sentences:
1. What the played move tried to do.
2. What it missed or conceded compared with the engine's move.
3. One concrete lesson for similar positions.
Be honest but constructive. Use only the facts above.

{rules}
"#,
        played_san = facts.played_san,
        played_uci = facts.played_uci,
        ply = facts.ply,
        before_fen = facts.before_fen,
        after_fen = facts.after_fen,
        best_san = facts.best_san,
        best_uci = facts.best_uci,
        eval_before = facts.eval_before,
        eval_after = facts.eval_after,
        loss_cp = facts.loss_cp,
        loss = facts.loss_cp as f64 / 100.0,
        classification = facts.classification,
        language = language,
        rules = honest_rules(language),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(uci: &str, score_cp: i32) -> Candidate {
        Candidate {
            uci: uci.to_string(),
            score_cp: Some(score_cp),
            mate_in: None,
            depth: 10,
            multipv: 1,
            pv: Vec::new(),
        }
    }

    #[test]
    fn formats_evaluations() {
        assert_eq!(format_eval(20), "+0.20");
        assert_eq!(format_eval(-120), "-1.20");
        assert_eq!(format_eval(10_000), "+M");
        assert_eq!(format_eval(-10_000), "-M");
    }

    #[test]
    fn converts_candidate_scores_to_white_perspective() {
        let white = candidate("e2e4", 30);
        let black = candidate("e7e5", 30);
        assert_eq!(candidate_white_cp(&white, Color::White), Some(30));
        assert_eq!(candidate_white_cp(&black, Color::Black), Some(-30));
    }

    #[test]
    fn explain_prompt_does_not_leak_a_move() {
        let game = Game::new();
        let prompt = explain_prompt(&game, Color::White, "+0.20", "English");
        assert!(prompt.contains("Do not recommend a move for the human"));
        assert!(prompt.contains("Engine evaluation for White: +0.20"));
        assert!(!prompt.contains("e2e4"));
        assert!(!prompt.contains("Nf3"));
    }

    #[test]
    fn hints_reveal_progressively() {
        let game = Game::new();
        let candidates = vec![
            candidate("e2e4", 30),
            candidate("d2d4", 20),
            candidate("g1f3", 10),
        ];
        let idea = hint_prompt(&game, Color::White, 1, &candidates, "+0.30", "English");
        assert!(idea.contains("LEVEL 1"));
        assert!(!idea.contains("e2e4"));
        assert!(!idea.contains("Nf3"));

        let names = hint_prompt(&game, Color::White, 2, &candidates, "+0.30", "English");
        assert!(names.contains("LEVEL 2"));
        assert!(names.contains("e4"));
        assert!(names.contains("d4"));
        assert!(names.contains("Nf3"));
        assert!(!names.contains("e2e4"));
        assert!(!names.contains("Engine's preferred move"));

        let best = hint_prompt(&game, Color::White, 3, &candidates, "+0.30", "English");
        assert!(best.contains("LEVEL 3"));
        assert!(best.contains("e4 (e2e4)"));
    }

    #[test]
    fn review_prompt_includes_engine_facts() {
        let facts = ReviewFacts {
            played_san: "Nf3".to_string(),
            played_uci: "g1f3".to_string(),
            best_san: "e4".to_string(),
            best_uci: "e2e4".to_string(),
            before_fen: "start".to_string(),
            after_fen: "after".to_string(),
            eval_before: "+0.20".to_string(),
            eval_after: "-0.40".to_string(),
            loss_cp: 60,
            classification: "inaccuracy".to_string(),
            ply: 1,
        };
        let prompt = review_prompt(&facts, "Estonian");
        assert!(prompt.contains("Nf3 (g1f3)"));
        assert!(prompt.contains("e4 (e2e4)"));
        assert!(prompt.contains("0.60"));
        assert!(prompt.contains("inaccuracy"));
        assert!(prompt.contains("Estonian"));
    }

    #[test]
    #[ignore = "requires Stockfish"]
    fn review_facts_use_engine_truth() {
        let mut game = Game::new();
        game.apply_san("f3").unwrap();
        let path = Stockfish::discover().unwrap();
        let mut engine = Stockfish::new(path).unwrap();
        let facts = review_facts(&mut engine, &game, Color::White).unwrap();
        assert_eq!(facts.played_san, "f3");
        assert_ne!(facts.best_uci, "f2f3");
        assert!(facts.loss_cp > 0);
        assert_ne!(facts.classification, "best");
    }
}
