//! Bluff-intensity guidance for agent prompts.
//!
//! The slider controls how freely the opponent may exaggerate evaluations and
//! plans. It never permits lies about the board, the rules or move legality.

pub fn instruction(intensity: u8) -> String {
    let intensity = intensity.min(100);
    let (label, guidance) = match intensity {
        0..=15 => (
            "honest",
            "State evaluations and plans as you believe them. Do not deliberately exaggerate or mislead.",
        ),
        16..=39 => (
            "mild showmanship",
            "You may add light theatrical spin, but keep numeric evaluations within about 0.5 pawns of the engine's candidate score and keep plans plausible.",
        ),
        40..=64 => (
            "theatrical but plausible",
            "You may bluff about evaluations and plans. Keep numeric claims within about 1.2 pawns of the engine's candidate score so the theater stays believable.",
        ),
        65..=84 => (
            "bold liar",
            "Exaggerate evaluations and plans freely. When you give a numeric evaluation, move it at least about 1.5 pawns away from the engine's candidate score; you may claim a much better or worse position. Stay coherent and in character.",
        ),
        _ => (
            "unreliable narrator",
            "You may say almost anything about evaluations and plans, including wild swings and false confidence. When you give a numeric evaluation, do not simply quote the engine; choose a dramatically different number or a bold verdict. Stay coherent and in character.",
        ),
    };
    format!(
        "BLUFF INTENSITY: {intensity}/100 ({label}). {guidance} Bluffing applies only to evaluations and plans, never to board facts, rules, legality, captures, checks, mate or the result."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intensity_is_bounded_and_described() {
        assert!(instruction(0).contains("0/100 (honest)"));
        assert!(instruction(50).contains("50/100 (theatrical but plausible)"));
        assert!(instruction(100).contains("100/100 (unreliable narrator)"));
        assert!(instruction(200).contains("100/100"));
    }

    #[test]
    fn rules_are_always_excluded() {
        for value in [0, 25, 50, 75, 100] {
            let text = instruction(value);
            assert!(text.contains("never to board facts, rules, legality"));
        }
    }
}
