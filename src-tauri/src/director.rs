//! Theatrical post-game recap ("Director's Cut").
//!
//! The narrative is invented, but the moves, evaluations, quotes and Truth Lens
//! notes are facts taken from the game and the Stockfish analysis. The module
//! prepares the facts, validates the model's JSON and attaches the verified
//! mini-board positions.

use std::collections::HashMap;

use gambiit_core::{Game, color_key};
use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisReport;
use crate::coach::format_eval;

#[derive(Debug, Clone)]
pub struct Quote {
    pub uci: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ChatLine {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub game: Game,
    pub quotes: Vec<Quote>,
    pub chat: Vec<ChatLine>,
    pub persona: String,
    pub model: String,
    pub ui_language: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectorMove {
    pub san: String,
    pub uci: String,
    pub from: String,
    pub to: String,
    pub color: String,
    pub ply: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectorFlag {
    pub san: String,
    pub ply: u32,
    pub claimed_eval: String,
    pub gap_cp: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectorAct {
    pub kind: String,
    pub title: String,
    pub text: String,
    pub ply: u32,
    pub san: String,
    pub color: String,
    pub fen: String,
    pub last_move: Option<DirectorMove>,
    pub eval_cp: i32,
    pub classification: Option<String>,
    pub quote: Option<String>,
    pub quote_persona: Option<String>,
    pub truth_lens: Vec<DirectorFlag>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectorCut {
    pub title: String,
    pub tagline: String,
    pub acts: Vec<DirectorAct>,
    pub closing: String,
    pub model: String,
    pub tokens: Option<u64>,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PlannedAct {
    pub kind: &'static str,
    pub ply: u32,
    pub range_start: u32,
    pub range_end: u32,
    pub san: String,
    pub color: String,
    pub fen: String,
    pub last_move: Option<DirectorMove>,
    pub eval_cp: i32,
    pub classification: Option<String>,
    pub quote: Option<String>,
    pub truth_lens: Vec<DirectorFlag>,
}

impl PlannedAct {
    fn label(&self) -> &'static str {
        match self.kind {
            "turningPoint" => "Turning point",
            "finalAct" => "Final act",
            _ => "Opening",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirectorPlan {
    pub prompt: String,
    pub acts: Vec<PlannedAct>,
    pub persona: String,
}

#[derive(Debug, Deserialize)]
struct Response {
    title: String,
    tagline: String,
    acts: Vec<ActResponse>,
    closing: String,
}

#[derive(Debug, Deserialize)]
struct ActResponse {
    title: String,
    text: String,
}

fn clean(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value.chars().take(max_chars).collect::<String>() + "…"
    }
}

fn validate_report(game: &Game, report: &AnalysisReport) -> Result<(), String> {
    let history = game.history();
    if report.positions.len() != history.len() {
        return Err("The analysis is stale. Run the analysis again.".to_string());
    }
    for (position, record) in report.positions.iter().zip(history) {
        if position.uci != record.uci || position.fen_after != record.fen_after {
            return Err("The analysis does not match the current game. Run it again.".to_string());
        }
    }
    Ok(())
}

fn key_plies(report: &AnalysisReport) -> (u32, u32, u32) {
    let len = report.positions.len() as u32;
    let opening = 6.min(len - 2);
    let final_ply = len;
    let start = opening + 1;
    let end = final_ply - 1;
    let mut turning = start;
    let mut best_score = i64::MIN;
    for ply in start..=end {
        let position = &report.positions[(ply - 1) as usize];
        let swing = (position.eval_after_cp as i64 - position.eval_before_cp as i64).abs();
        let score = position.loss_cp as i64 * 2 + swing;
        if score > best_score {
            best_score = score;
            turning = ply;
        }
    }
    (opening, turning, final_ply)
}

fn quote_for_range(context: &Context, start: u32, end: u32) -> Option<String> {
    let quotes = context
        .quotes
        .iter()
        .map(|quote| (quote.uci.as_str(), quote.text.as_str()))
        .collect::<HashMap<_, _>>();
    if quotes.is_empty() {
        return None;
    }
    let agent_color = color_key(!context.game.human_color());
    let history = context.game.history();
    let range_start = start.saturating_sub(1) as usize;
    let range_end = (end as usize).min(history.len());

    let mut found = None;
    for record in &history[range_start..range_end] {
        if record.color == agent_color
            && let Some(text) = quotes.get(record.uci.as_str())
        {
            found = Some((*text).to_string());
        }
    }
    if found.is_none() {
        for record in history.iter().skip(range_end) {
            if record.color == agent_color
                && let Some(text) = quotes.get(record.uci.as_str())
            {
                found = Some((*text).to_string());
                break;
            }
        }
    }
    if found.is_none() {
        for record in history.iter().take(range_start).rev() {
            if record.color == agent_color
                && let Some(text) = quotes.get(record.uci.as_str())
            {
                found = Some((*text).to_string());
                break;
            }
        }
    }
    found.map(|text| truncate(&text, 500))
}

fn truth_lens_for_range(report: &AnalysisReport, start: u32, end: u32) -> Vec<DirectorFlag> {
    report
        .positions
        .iter()
        .skip(start.saturating_sub(1) as usize)
        .take((end - start + 1) as usize)
        .filter(|position| position.claim_mismatch)
        .take(3)
        .map(|position| DirectorFlag {
            san: position.san.clone(),
            ply: position.ply,
            claimed_eval: position
                .claimed_eval
                .clone()
                .unwrap_or_else(|| "—".to_string()),
            gap_cp: position.claim_gap_cp.unwrap_or(0),
        })
        .collect()
}

fn planned_act(
    context: &Context,
    report: &AnalysisReport,
    kind: &'static str,
    ply: u32,
    range_start: u32,
    range_end: u32,
) -> PlannedAct {
    let position = &report.positions[(ply - 1) as usize];
    let record = &context.game.history()[(ply - 1) as usize];
    let last_move = Some(DirectorMove {
        san: record.san.clone(),
        uci: record.uci.clone(),
        from: record.from.clone(),
        to: record.to.clone(),
        color: record.color.clone(),
        ply: record.ply,
    });
    PlannedAct {
        kind,
        ply,
        range_start,
        range_end,
        san: record.san.clone(),
        color: record.color.clone(),
        fen: record.fen_after.clone(),
        last_move,
        eval_cp: position.eval_after_cp,
        classification: Some(position.classification.clone()),
        quote: quote_for_range(context, range_start, range_end),
        truth_lens: truth_lens_for_range(report, range_start, range_end),
    }
}

fn act_facts(act: &PlannedAct) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} (plies {}-{}); the act's board is after ply {}: {} by {}.",
        act.label(),
        act.range_start,
        act.range_end,
        act.ply,
        act.san,
        act.color
    ));
    lines.push(format!(
        "Evaluation after that position: {}; move classification: {}.",
        format_eval(act.eval_cp),
        act.classification.as_deref().unwrap_or("—")
    ));
    if let Some(quote) = &act.quote {
        lines.push(format!("Agent quote from this act: \"{quote}\""));
    }
    if !act.truth_lens.is_empty() {
        let flags = act
            .truth_lens
            .iter()
            .map(|flag| {
                format!(
                    "{} claimed {} (gap {:.2} pawns)",
                    flag.san,
                    flag.claimed_eval,
                    flag.gap_cp as f64 / 100.0
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        lines.push(format!("Truth Lens flags: {flags}."));
    }
    lines.join("\n")
}

fn build_prompt(context: &Context, report: &AnalysisReport, acts: &[PlannedAct]) -> String {
    let human_color = context.game.human_color();
    let chat = if context.chat.is_empty() {
        "(none)".to_string()
    } else {
        context
            .chat
            .iter()
            .rev()
            .take(8)
            .rev()
            .map(|line| {
                let speaker = if line.role == "user" {
                    "Human"
                } else {
                    context.persona.as_str()
                };
                format!("{speaker}: {}", truncate(&line.text, 300))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let facts = acts.iter().map(act_facts).collect::<Vec<_>>().join("\n\n");

    format!(
        r#"You are the director of a theatrical post-game recap for a chess game.
The board, result, moves, evaluations, classifications, quotes and Truth Lens notes below are FACTS. The story is theater: be dramatic, but never invent moves, results, evaluations, quotes or claims, and never change the result. Do not present the story as objective analysis.

GAME
Result: {result}
White: {white}
Black: {black}
Opponent persona: {persona} ({model})
Moves (SAN): {moves}
Accuracy: White {white_accuracy:.1}%, Black {black_accuracy:.1}%.
Blunders / mistakes / inaccuracies: White {white_blunders}/{white_mistakes}/{white_inaccuracies}, Black {black_blunders}/{black_mistakes}/{black_inaccuracies}.
Table talk:
{chat}

KEY MOMENTS
{facts}

TASK
Write a three-act theatrical recap in {language}. Use the acts in the order above: opening, turning point, final act. The human is the protagonist; the opponent persona is the antagonist. You may reference the supplied quotes but must not invent new ones. If a Truth Lens note says a claim was flagged, you may call it a bluff or a tall tale, but do not pretend the claim was true.

Return ONLY one JSON object, no markdown fences, no prose before or after:
{{
  "title": "Short evocative title",
  "tagline": "One sentence",
  "acts": [
    {{ "title": "Evocative act title", "text": "2-4 sentences" }},
    {{ "title": "Evocative act title", "text": "2-4 sentences" }},
    {{ "title": "Evocative act title", "text": "2-4 sentences" }}
  ],
  "closing": "1-2 sentences"
}}

Exactly three acts, in order. Write in {language}. Do not mention hidden chain-of-thought.
"#,
        result = context.game.status_text(),
        white = if human_color == shakmaty::Color::White {
            "the human".to_string()
        } else {
            format!("{} ({})", context.persona, context.model)
        },
        black = if human_color == shakmaty::Color::Black {
            "the human".to_string()
        } else {
            format!("{} ({})", context.persona, context.model)
        },
        persona = context.persona,
        model = context.model,
        moves = context.game.pgn_moves(),
        white_accuracy = report.white_accuracy,
        black_accuracy = report.black_accuracy,
        white_blunders = report.white_blunders,
        white_mistakes = report.white_mistakes,
        white_inaccuracies = report.white_inaccuracies,
        black_blunders = report.black_blunders,
        black_mistakes = report.black_mistakes,
        black_inaccuracies = report.black_inaccuracies,
        chat = chat,
        facts = facts,
        language = context.ui_language,
    )
}

pub fn prepare(context: &Context, report: &AnalysisReport) -> Result<DirectorPlan, String> {
    validate_report(&context.game, report)?;
    if report.positions.len() < 8 {
        return Err("The Director's Cut needs at least eight plies.".to_string());
    }
    let (opening, turning, final_ply) = key_plies(report);
    let acts = vec![
        planned_act(context, report, "opening", opening, 1, opening),
        planned_act(
            context,
            report,
            "turningPoint",
            turning,
            opening + 1,
            turning,
        ),
        planned_act(
            context,
            report,
            "finalAct",
            final_ply,
            turning + 1,
            final_ply,
        ),
    ];
    Ok(DirectorPlan {
        prompt: build_prompt(context, report, &acts),
        acts,
        persona: context.persona.clone(),
    })
}

fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start).then_some(&text[start..=end])
}

pub fn retry_prompt(error: &str) -> String {
    format!(
        "Your previous reply could not be parsed as the required Director's Cut JSON: {error}. Return ONLY the JSON object with title, tagline, exactly three acts (each with a title and text), and closing. No markdown fences, no prose."
    )
}

pub fn finish(
    plan: &DirectorPlan,
    text: &str,
    model: &str,
    tokens: Option<u64>,
    cost: Option<f64>,
) -> Result<DirectorCut, String> {
    let json = extract_json(text).ok_or_else(|| "No JSON object was found.".to_string())?;
    let response: Response = serde_json::from_str(json)
        .map_err(|error| format!("The Director's Cut JSON was invalid: {error}"))?;
    let title = clean(&response.title, 140);
    let tagline = clean(&response.tagline, 320);
    let closing = clean(&response.closing, 600);
    if title.is_empty() || tagline.is_empty() || closing.is_empty() {
        return Err("The Director's Cut is missing a title, tagline or closing.".to_string());
    }
    if response.acts.len() != 3 {
        return Err("The Director's Cut needs exactly three acts.".to_string());
    }
    let mut acts = Vec::with_capacity(3);
    for (planned, response_act) in plan.acts.iter().zip(response.acts.iter()) {
        let act_title = clean(&response_act.title, 140);
        let act_text = clean(&response_act.text, 1_800);
        if act_title.is_empty() || act_text.is_empty() {
            return Err("A Director's Cut act is missing its title or text.".to_string());
        }
        acts.push(DirectorAct {
            kind: planned.kind.to_string(),
            title: act_title,
            text: act_text,
            ply: planned.ply,
            san: planned.san.clone(),
            color: planned.color.clone(),
            fen: planned.fen.clone(),
            last_move: planned.last_move.clone(),
            eval_cp: planned.eval_cp,
            classification: planned.classification.clone(),
            quote: planned.quote.clone(),
            quote_persona: planned.quote.as_ref().map(|_| plan.persona.clone()),
            truth_lens: planned.truth_lens.clone(),
        });
    }
    Ok(DirectorCut {
        title,
        tagline,
        acts,
        closing,
        model: model.to_string(),
        tokens,
        cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::AnalysisPly;

    fn dummy_report(len: usize) -> AnalysisReport {
        let positions = (1..=len)
            .map(|ply| AnalysisPly {
                ply: ply as u32,
                san: format!("m{ply}"),
                uci: format!("a1a{}", (ply % 8) + 1),
                color: if ply % 2 == 0 { "black" } else { "white" }.to_string(),
                fen_before: "before".to_string(),
                fen_after: "after".to_string(),
                eval_before_cp: 0,
                eval_after_cp: 0,
                best_uci: None,
                best_san: None,
                loss_cp: if ply == len / 2 { 250 } else { 10 },
                classification: "good".to_string(),
                claimed_eval: None,
                claim_gap_cp: None,
                claim_mismatch: false,
            })
            .collect();
        AnalysisReport {
            depth: 10,
            initial_eval_cp: 0,
            positions,
            white_accuracy: 80.0,
            black_accuracy: 70.0,
            white_blunders: 0,
            black_blunders: 0,
            white_mistakes: 0,
            black_mistakes: 0,
            white_inaccuracies: 0,
            black_inaccuracies: 0,
            truth_lens_flags: 0,
        }
    }

    fn play_game() -> Game {
        let mut game = Game::new();
        for mv in ["e4", "e5", "Nf3", "Nc6", "Bb5", "a6", "Ba4", "Nf6"] {
            game.apply_san(mv).unwrap();
        }
        game
    }

    fn report_for_game(game: &Game) -> AnalysisReport {
        let positions = game
            .history()
            .iter()
            .enumerate()
            .map(|(index, record)| AnalysisPly {
                ply: record.ply,
                san: record.san.clone(),
                uci: record.uci.clone(),
                color: record.color.clone(),
                fen_before: if index == 0 {
                    game.initial_fen().to_string()
                } else {
                    game.history()[index - 1].fen_after.clone()
                },
                fen_after: record.fen_after.clone(),
                eval_before_cp: 0,
                eval_after_cp: 0,
                best_uci: None,
                best_san: None,
                loss_cp: if index == 4 { 200 } else { 10 },
                classification: "good".to_string(),
                claimed_eval: None,
                claim_gap_cp: None,
                claim_mismatch: false,
            })
            .collect();
        AnalysisReport {
            depth: 10,
            initial_eval_cp: 0,
            positions,
            white_accuracy: 80.0,
            black_accuracy: 70.0,
            white_blunders: 0,
            black_blunders: 0,
            white_mistakes: 0,
            black_mistakes: 0,
            white_inaccuracies: 0,
            black_inaccuracies: 0,
            truth_lens_flags: 0,
        }
    }

    #[test]
    fn key_plies_are_ordered_and_inside_the_game() {
        let report = dummy_report(40);
        let (opening, turning, final_ply) = key_plies(&report);
        assert_eq!(opening, 6);
        assert!(turning > opening);
        assert!(turning < final_ply);
        assert_eq!(final_ply, 40);
    }

    #[test]
    fn finish_builds_three_acts() {
        let plan = DirectorPlan {
            prompt: String::new(),
            persona: "Trickster".to_string(),
            acts: (0..3)
                .map(|index| PlannedAct {
                    kind: match index {
                        1 => "turningPoint",
                        2 => "finalAct",
                        _ => "opening",
                    },
                    ply: index + 1,
                    range_start: index + 1,
                    range_end: index + 1,
                    san: "e4".to_string(),
                    color: "white".to_string(),
                    fen: "fen".to_string(),
                    last_move: None,
                    eval_cp: 20,
                    classification: Some("good".to_string()),
                    quote: Some("A quote.".to_string()),
                    truth_lens: Vec::new(),
                })
                .collect(),
        };
        let cut = finish(
            &plan,
            r#"{"title":"The Opening","tagline":"A game of nerves.","acts":[{"title":"First","text":"One."},{"title":"Second","text":"Two."},{"title":"Third","text":"Three."}],"closing":"Curtain."}"#,
            "model",
            Some(10),
            None,
        )
        .unwrap();
        assert_eq!(cut.title, "The Opening");
        assert_eq!(cut.acts.len(), 3);
        assert_eq!(cut.acts[1].kind, "turningPoint");
        assert_eq!(cut.acts[0].quote_persona.as_deref(), Some("Trickster"));
    }

    #[test]
    fn finish_rejects_wrong_act_count() {
        let plan = DirectorPlan {
            prompt: String::new(),
            persona: "Trickster".to_string(),
            acts: Vec::new(),
        };
        let error = finish(
            &plan,
            r#"{"title":"T","tagline":"G","acts":[{"title":"A","text":"B"}],"closing":"C"}"#,
            "model",
            None,
            None,
        )
        .unwrap_err();
        assert!(error.contains("exactly three"));
    }

    #[test]
    fn prepare_rejects_a_stale_report() {
        let game = play_game();
        let mut report = report_for_game(&game);
        report.positions[0].uci = "h2h4".to_string();
        let context = Context {
            game,
            quotes: Vec::new(),
            chat: Vec::new(),
            persona: "Trickster".to_string(),
            model: "model".to_string(),
            ui_language: "English".to_string(),
        };
        assert!(prepare(&context, &report).unwrap_err().contains("analysis"));
    }

    #[test]
    fn prompt_is_factual_and_asks_for_three_acts() {
        let game = play_game();
        let report = report_for_game(&game);
        let context = Context {
            game,
            quotes: vec![Quote {
                uci: "e2e4".to_string(),
                text: "A bold opening.".to_string(),
            }],
            chat: Vec::new(),
            persona: "Trickster".to_string(),
            model: "model".to_string(),
            ui_language: "Estonian".to_string(),
        };
        let plan = prepare(&context, &report).unwrap();
        assert!(plan.prompt.contains("Exactly three acts"));
        assert!(plan.prompt.contains("Estonian"));
        assert!(plan.prompt.contains("Trickster"));
        assert!(plan.prompt.contains("Truth Lens"));
        assert_eq!(plan.acts.len(), 3);
    }
}
