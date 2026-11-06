//! Bounded persona move style.
//!
//! Stockfish still defines the candidate pool and the best evaluation. A
//! persona may only reorder candidates inside a small centipawn margin, so the
//! opponent gains character without silently losing playing strength.

use gambiit_core::{Game, MoveInfo};
use gambiit_engine::Candidate;

use crate::CustomPersona;

#[derive(Debug, Clone, Copy)]
struct StyleProfile {
    aggression: f64,
    safety: f64,
    showmanship: f64,
    chaos: f64,
    risk: f64,
}

#[derive(Debug, Clone, Copy)]
struct MoveFeatures {
    capture: bool,
    check: bool,
    castle: bool,
    promotion: bool,
    pawn: bool,
    knight: bool,
    queen: bool,
    central: bool,
    king_side: bool,
    sharp: bool,
    loss_cp: i32,
    index: usize,
}

pub fn tolerance(mode: &str) -> i32 {
    match mode {
        "off" => 0,
        "theatrical" => 40,
        "strong" => 70,
        _ => 20,
    }
}

pub fn instruction_custom(persona: &CustomPersona) -> String {
    format!(
        "Persona move style: custom mix (aggression {aggression}/10, theatricality {theatricality}/10, humour {humor}/10, deception {deception}/10). Prefer the first candidate when it fits this style and stays inside the allowed strength margin.",
        aggression = persona.aggression,
        theatricality = persona.theatricality,
        humor = persona.humor,
        deception = persona.deception,
    )
}

pub fn instruction(persona: &str) -> String {
    match persona.to_ascii_lowercase().as_str() {
        "panicker" => "Persona move style: cautious. Prefer castling, king safety and solid moves; avoid unnecessary sacrifices, sharp checks and pawn adventures. The first candidate is the safest style fit.".to_string(),
        "trickster" => "Persona move style: trickster. Prefer checks, captures, traps, pawn storms and sound sacrifices. The first candidate is the most theatrical style fit; tricks are welcome, but stay inside the candidate pool.".to_string(),
        "cold calculator" => String::new(),
        "coach" => "Persona move style: principled coach. Prefer development, central control and clean plans. The first candidate is the style fit.".to_string(),
        "chaotic" => "Persona move style: chaos. Prefer surprising and offbeat candidates while staying sound. The first candidate is the style fit.".to_string(),
        "theater master" => "Persona move style: theatrical attacker. Prefer checks, king-side pressure and dramatic piece activity. The first candidate is the style fit.".to_string(),
        _ => String::new(),
    }
}

fn custom_profile(persona: &CustomPersona) -> StyleProfile {
    StyleProfile {
        aggression: (persona.aggression as f64 - 5.0) * 1.8,
        safety: (10.0 - persona.aggression as f64) * 0.5 + persona.warmth as f64 * 0.2 - 1.0,
        showmanship: persona.theatricality as f64 * 0.8,
        chaos: persona.humor as f64 * 0.4 + persona.deception as f64 * 0.3,
        risk: persona.aggression as f64 * 0.6 + persona.confidence as f64 * 0.2 - 3.0,
    }
}

fn profile(persona: &str) -> Option<StyleProfile> {
    match persona.to_ascii_lowercase().as_str() {
        "panicker" => Some(StyleProfile {
            aggression: -4.0,
            safety: 8.0,
            showmanship: 0.0,
            chaos: -2.0,
            risk: -6.0,
        }),
        "trickster" => Some(StyleProfile {
            aggression: 8.0,
            safety: -2.0,
            showmanship: 6.0,
            chaos: 5.0,
            risk: 6.0,
        }),
        "theater master" => Some(StyleProfile {
            aggression: 5.0,
            safety: 0.0,
            showmanship: 8.0,
            chaos: 2.0,
            risk: 3.0,
        }),
        "coach" => Some(StyleProfile {
            aggression: -1.0,
            safety: 5.0,
            showmanship: 0.0,
            chaos: 0.0,
            risk: -2.0,
        }),
        "chaotic" => Some(StyleProfile {
            aggression: 4.0,
            safety: -3.0,
            showmanship: 4.0,
            chaos: 9.0,
            risk: 5.0,
        }),
        "cold calculator" => None,
        _ => None,
    }
}

fn loss_cp(best: &Candidate, candidate: &Candidate) -> Option<i32> {
    match (best.mate_in, candidate.mate_in) {
        (Some(best_mate), Some(candidate_mate))
            if best_mate.signum() == candidate_mate.signum() =>
        {
            Some(0)
        }
        (Some(_), _) | (_, Some(_)) => None,
        (None, None) => {
            let best_score = best.score_cp?;
            let score = candidate.score_cp?;
            Some((best_score - score).max(0))
        }
    }
}

fn features(
    legal: &[MoveInfo],
    candidate: &Candidate,
    index: usize,
    best: &Candidate,
    tolerance: i32,
) -> Option<MoveFeatures> {
    let info = legal.iter().find(|mv| mv.uci == candidate.uci)?;
    let loss = loss_cp(best, candidate)?;
    if loss > tolerance {
        return None;
    }
    let file = info.to.chars().next()?;
    let rank = info.to.chars().nth(1)?;
    Some(MoveFeatures {
        capture: info.captured.is_some(),
        check: info.san.ends_with('+') || info.san.ends_with('#'),
        castle: info.is_castle,
        promotion: info.promotion.is_some(),
        pawn: info.piece == "p",
        knight: info.piece == "n",
        queen: info.piece == "q",
        central: matches!(file, 'c' | 'd' | 'e' | 'f') && matches!(rank, '3' | '4' | '5' | '6'),
        king_side: matches!(file, 'f' | 'g' | 'h'),
        sharp: loss > 0 && info.captured.is_none(),
        loss_cp: loss,
        index,
    })
}

fn score(profile: StyleProfile, features: MoveFeatures, tolerance: i32) -> f64 {
    let loss_ratio = if tolerance > 0 {
        features.loss_cp as f64 / tolerance as f64
    } else {
        0.0
    };
    let mut value = 0.0;
    if features.capture {
        value += profile.aggression * 1.2 + profile.showmanship * 0.8 + profile.safety * 0.3;
    }
    if features.check {
        value += profile.aggression * 1.0 + profile.showmanship * 1.5 + profile.risk * 0.5;
    }
    if features.castle {
        value += profile.safety * 2.0 + profile.showmanship * 0.3;
    }
    if features.promotion {
        value += profile.aggression * 1.5 + profile.showmanship * 1.5;
    }
    if features.pawn {
        value += profile.aggression * 0.3 - profile.safety * 0.2;
    }
    if features.knight || features.queen {
        value += profile.chaos * 0.4 + profile.showmanship * 0.2;
    }
    if features.central {
        value += profile.safety * 0.2;
    }
    if features.king_side {
        value += profile.aggression * 0.4 + profile.showmanship * 0.5;
    }
    if features.sharp {
        value += profile.risk * 1.5 + profile.showmanship * 1.0 + profile.aggression * 0.8;
    }
    value -= loss_ratio * 0.8;
    if features.index == 0 {
        value += 0.5;
    }
    value
}

/// Return the index of the candidate that best fits the persona's style.
/// The returned candidate is always within `tolerance` centipawns of the best.
pub fn choose(
    game: &Game,
    candidates: &[Candidate],
    persona: &str,
    tolerance: i32,
) -> Option<usize> {
    choose_with_profile(game, candidates, profile(persona)?, tolerance)
}

pub fn choose_custom(
    game: &Game,
    candidates: &[Candidate],
    persona: &CustomPersona,
    tolerance: i32,
) -> Option<usize> {
    choose_with_profile(game, candidates, custom_profile(persona), tolerance)
}

fn choose_with_profile(
    game: &Game,
    candidates: &[Candidate],
    style: StyleProfile,
    tolerance: i32,
) -> Option<usize> {
    if tolerance <= 0 || candidates.len() < 2 {
        return None;
    }
    let best = candidates.first()?;
    let legal = game.legal_moves();
    let mut best_choice: Option<(usize, f64)> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(move_features) = features(&legal, candidate, index, best, tolerance) else {
            continue;
        };
        let value = score(style, move_features, tolerance);
        if best_choice.is_none_or(|(_, current)| value > current) {
            best_choice = Some((index, value));
        }
    }
    best_choice.map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_modes_are_bounded() {
        assert_eq!(tolerance("off"), 0);
        assert_eq!(tolerance("subtle"), 20);
        assert_eq!(tolerance("theatrical"), 40);
        assert_eq!(tolerance("strong"), 70);
    }

    #[test]
    fn cold_calculator_has_no_style() {
        assert!(instruction("Cold Calculator").is_empty());
        assert!(choose(&Game::new(), &[], "Cold Calculator", 20).is_none());
    }

    #[test]
    fn trickster_and_panicker_have_instructions() {
        assert!(instruction("Trickster").contains("sacrifices"));
        assert!(instruction("Panicker").contains("cautious"));
    }

    #[test]
    fn custom_persona_has_style_guidance() {
        let persona = CustomPersona {
            theatricality: 9,
            aggression: 8,
            deception: 7,
            ..Default::default()
        };
        let instruction = instruction_custom(&persona);
        assert!(instruction.contains("aggression 8/10"));
        assert!(instruction.contains("theatricality 9/10"));
        assert!(instruction.contains("deception 7/10"));
    }

    #[test]
    fn personas_choose_different_near_best_moves() {
        let game = Game::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let candidates = vec![
            Candidate {
                uci: "e1g1".to_string(),
                score_cp: Some(20),
                mate_in: None,
                depth: 12,
                multipv: 1,
                pv: Vec::new(),
            },
            Candidate {
                uci: "a1a8".to_string(),
                score_cp: Some(10),
                mate_in: None,
                depth: 12,
                multipv: 2,
                pv: Vec::new(),
            },
        ];
        assert_eq!(choose(&game, &candidates, "Cold Calculator", 20), None);
        assert_eq!(choose(&game, &candidates, "Panicker", 20), Some(0));
        assert_eq!(choose(&game, &candidates, "Trickster", 20), Some(1));
    }
}
