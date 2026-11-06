//! Local model leaderboard built from saved games.

use crate::storage;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardRow {
    pub model: String,
    pub games: u32,
    pub finished: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub win_rate: f64,
    pub moves: u32,
    pub analyzed_games: u32,
    pub accuracy: Option<f64>,
    pub blunders: u32,
    pub mistakes: u32,
    pub inaccuracies: u32,
    pub truth_lens_flags: u32,
    pub bluff_rate: Option<f64>,
    pub avg_response_ms: Option<u64>,
    pub tokens: u64,
    pub cost: f64,
    pub difficulties: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Leaderboard {
    pub rows: Vec<LeaderboardRow>,
    pub difficulties: Vec<String>,
}

#[derive(Default)]
struct Accumulator {
    games: u32,
    finished: u32,
    wins: u32,
    draws: u32,
    losses: u32,
    moves: u32,
    analyzed_games: u32,
    analyzed_agent_moves: u32,
    accuracy_weighted: f64,
    blunders: u32,
    mistakes: u32,
    inaccuracies: u32,
    truth_lens_flags: u32,
    response_sum_ms: u64,
    response_count: u32,
    tokens: u64,
    cost: f64,
    difficulties: BTreeSet<String>,
}

pub fn build(difficulty: Option<&str>) -> Result<Leaderboard, String> {
    Ok(build_from_games(storage::load_all()?, difficulty))
}

fn build_from_games(games: Vec<storage::SavedGame>, difficulty: Option<&str>) -> Leaderboard {
    let mut all_difficulties = BTreeSet::new();
    let mut models: BTreeMap<String, Accumulator> = BTreeMap::new();

    for game in games {
        let model = game.settings.model.trim().to_string();
        if model.is_empty() {
            continue;
        }
        let difficulty_id = game.settings.difficulty.clone();
        all_difficulties.insert(difficulty_id.clone());
        if let Some(filter) = difficulty
            && !filter.is_empty()
            && filter != "all"
            && difficulty_id != filter
        {
            continue;
        }

        let bucket = models.entry(model.clone()).or_default();
        bucket.games += 1;
        bucket.moves += game.moves.len() as u32;
        bucket.difficulties.insert(difficulty_id);

        let agent_is_white = game.human_color.eq_ignore_ascii_case("black");
        match game.result.as_str() {
            "1-0" => {
                bucket.finished += 1;
                if agent_is_white {
                    bucket.wins += 1;
                } else {
                    bucket.losses += 1;
                }
            }
            "0-1" => {
                bucket.finished += 1;
                if agent_is_white {
                    bucket.losses += 1;
                } else {
                    bucket.wins += 1;
                }
            }
            "1/2-1/2" => {
                bucket.finished += 1;
                bucket.draws += 1;
            }
            _ => {}
        }

        for card in &game.commentary {
            if card.elapsed_ms > 0 {
                bucket.response_sum_ms += card.elapsed_ms;
                bucket.response_count += 1;
            }
            if let Some(tokens) = card.tokens {
                bucket.tokens += tokens;
            }
            if let Some(cost) = card.cost {
                bucket.cost += cost;
            }
        }

        if let Some(summary) = &game.analysis {
            bucket.analyzed_games += 1;
            bucket.analyzed_agent_moves += summary.agent_moves;
            bucket.accuracy_weighted += summary.agent_accuracy * summary.agent_moves as f64;
            bucket.blunders += summary.agent_blunders;
            bucket.mistakes += summary.agent_mistakes;
            bucket.inaccuracies += summary.agent_inaccuracies;
            bucket.truth_lens_flags += summary.truth_lens_flags;
        }
    }

    let mut rows: Vec<LeaderboardRow> = models
        .into_iter()
        .map(|(model, bucket)| {
            let accuracy = (bucket.analyzed_agent_moves > 0)
                .then(|| bucket.accuracy_weighted / bucket.analyzed_agent_moves as f64);
            let bluff_rate = (bucket.analyzed_agent_moves > 0).then(|| {
                bucket.truth_lens_flags as f64 / bucket.analyzed_agent_moves as f64 * 100.0
            });
            let win_rate = if bucket.finished > 0 {
                bucket.wins as f64 / bucket.finished as f64 * 100.0
            } else {
                0.0
            };
            LeaderboardRow {
                model,
                games: bucket.games,
                finished: bucket.finished,
                wins: bucket.wins,
                draws: bucket.draws,
                losses: bucket.losses,
                win_rate,
                moves: bucket.moves,
                analyzed_games: bucket.analyzed_games,
                accuracy,
                blunders: bucket.blunders,
                mistakes: bucket.mistakes,
                inaccuracies: bucket.inaccuracies,
                truth_lens_flags: bucket.truth_lens_flags,
                bluff_rate,
                avg_response_ms: (bucket.response_count > 0)
                    .then(|| bucket.response_sum_ms / bucket.response_count as u64),
                tokens: bucket.tokens,
                cost: bucket.cost,
                difficulties: bucket.difficulties.into_iter().collect(),
            }
        })
        .collect();

    rows.sort_by(|left, right| {
        let left_finished = left.finished > 0;
        let right_finished = right.finished > 0;
        right_finished
            .cmp(&left_finished)
            .then_with(|| right.win_rate.total_cmp(&left.win_rate))
            .then_with(|| right.games.cmp(&left.games))
            .then_with(|| left.model.cmp(&right.model))
    });

    Leaderboard {
        rows,
        difficulties: all_difficulties.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::AnalysisSummary;
    use crate::storage::SavedGame;
    use crate::{ChatMessage, CommentaryCard, Settings};

    fn game(model: &str, difficulty: &str, result: &str, human: &str) -> SavedGame {
        SavedGame {
            id: format!("{model}-{result}"),
            name: "Test".to_string(),
            created_at: 1,
            updated_at: 1,
            human_color: human.to_string(),
            initial_fen: String::new(),
            moves: Vec::new(),
            commentary: vec![CommentaryCard {
                san: "e4".to_string(),
                uci: "e2e4".to_string(),
                comment: "A move.".to_string(),
                eval: Some("+0.1".to_string()),
                candidates: Vec::new(),
                plan: None,
                tokens: Some(100),
                cost: Some(0.01),
                elapsed_ms: 1_000,
            }],
            chat: Vec::<ChatMessage>::new(),
            clock: None,
            analysis: None,
            settings: Settings {
                model: model.to_string(),
                difficulty: difficulty.to_string(),
                ..Default::default()
            },
            result: result.to_string(),
            termination: "ongoing".to_string(),
        }
    }

    #[test]
    fn aggregates_results_and_analysis() {
        let mut first = game("model-a", "intermediate", "1-0", "black");
        first.analysis = Some(AnalysisSummary {
            depth: 12,
            analyzed_plies: 4,
            agent_accuracy: 80.0,
            agent_moves: 10,
            agent_blunders: 1,
            agent_mistakes: 2,
            agent_inaccuracies: 3,
            truth_lens_flags: 2,
        });
        let second = game("model-a", "intermediate", "1-0", "white");
        let third = game("model-b", "master", "1/2-1/2", "white");
        let board = build_from_games(vec![first, second, third], None);
        assert_eq!(board.rows.len(), 2);
        let model_a = board
            .rows
            .iter()
            .find(|row| row.model == "model-a")
            .unwrap();
        assert_eq!(model_a.games, 2);
        assert_eq!(model_a.finished, 2);
        assert_eq!(model_a.wins, 1);
        assert_eq!(model_a.losses, 1);
        assert_eq!(model_a.win_rate, 50.0);
        assert_eq!(model_a.analyzed_games, 1);
        assert_eq!(model_a.accuracy, Some(80.0));
        assert_eq!(model_a.bluff_rate, Some(20.0));
        assert_eq!(model_a.tokens, 200);
        assert_eq!(model_a.avg_response_ms, Some(1_000));

        let filtered = build_from_games(
            vec![
                game("model-a", "intermediate", "1-0", "black"),
                game("model-b", "master", "0-1", "white"),
            ],
            Some("master"),
        );
        assert_eq!(filtered.rows.len(), 1);
        assert_eq!(filtered.rows[0].model, "model-b");
        assert_eq!(filtered.difficulties, vec!["intermediate", "master"]);
    }
}
