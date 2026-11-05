//! PGN export and a tolerant PGN importer for local games.

use crate::{CommentaryCard, Settings};
use gambiit_core::{Game, GameStatus, color_key};
use shakmaty::Color;

pub struct Imported {
    pub game: Game,
    pub commentary: Vec<CommentaryCard>,
    pub name: String,
}

pub fn result(game: &Game) -> &'static str {
    match game.status() {
        GameStatus::Checkmate { winner } => {
            if winner == Color::White {
                "1-0"
            } else {
                "0-1"
            }
        }
        GameStatus::Resigned { winner } => {
            if winner == Color::White {
                "1-0"
            } else {
                "0-1"
            }
        }
        status if status.is_over() => "1/2-1/2",
        _ => "*",
    }
}

fn escape_tag(value: &str) -> String {
    value.replace('"', "'")
}

pub fn export(game: &Game, settings: &Settings) -> String {
    let human_white = game.human_color() == Color::White;
    let human_name = "You";
    let agent_name = format!("{} ({})", settings.persona, settings.model);
    let (white, black) = if human_white {
        (human_name, agent_name.as_str())
    } else {
        (agent_name.as_str(), human_name)
    };
    let mut out = String::new();
    out.push_str("[Event \"Gambiit\"]\n");
    out.push_str("[Site \"local\"]\n");
    out.push_str("[Date \"????.??.??\"]\n");
    out.push_str("[Round \"-\"]\n");
    out.push_str(&format!("[White \"{}\"]\n", escape_tag(white)));
    out.push_str(&format!("[Black \"{}\"]\n", escape_tag(black)));
    out.push_str(&format!("[Result \"{}\"]\n", result(game)));
    out.push_str(&format!("[Model \"{}\"]\n", escape_tag(&settings.model)));
    out.push_str(&format!(
        "[HumanColor \"{}\"]\n",
        color_key(game.human_color())
    ));
    out.push_str("[TimeControl \"-\"]\n\n");

    for (index, record) in game.history().iter().enumerate() {
        if index % 2 == 0 {
            out.push_str(&format!("{}. ", index / 2 + 1));
        }
        out.push_str(&record.san);
        if let Some(comment) = &record.comment {
            let comment = comment.trim();
            if !comment.is_empty() {
                out.push_str(" {");
                out.push_str(&comment.replace('}', ")"));
                out.push('}');
            }
        }
        out.push(' ');
    }
    out.push_str(result(game));
    out.push('\n');
    out
}

fn tag_value(line: &str, tag: &str) -> Option<String> {
    let prefix = format!("[{tag} \"");
    line.trim()
        .strip_prefix(&prefix)?
        .strip_suffix("\"]")
        .map(|value| value.replace("\\\"", "\""))
}

fn apply_token(token: &str, game: &mut Game) -> Result<(), String> {
    let mut token = token.trim();
    if token.is_empty()
        || token == "*"
        || token == "1-0"
        || token == "0-1"
        || token == "1/2-1/2"
        || token.starts_with('$')
    {
        return Ok(());
    }

    if let Some(dot) = token.find('.')
        && token[..dot]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        token = &token[dot + 1..];
    }
    token = token.trim_start_matches('.');
    if token.is_empty()
        || token
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return Ok(());
    }

    let token = token.trim_end_matches(['!', '?']);
    if token.is_empty() {
        return Ok(());
    }
    game.apply_san(token)
        .map_err(|error| format!("Could not import move '{token}': {error}"))?;
    Ok(())
}

pub fn parse(input: &str) -> Result<Imported, String> {
    let mut fen = None;
    let mut human_color = None;
    let mut name = "Imported game".to_string();

    for line in input.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') {
            continue;
        }
        if let Some(value) = tag_value(trimmed, "FEN") {
            fen = Some(value);
        }
        if let Some(value) = tag_value(trimmed, "HumanColor") {
            human_color = Some(value);
        }
        if let Some(value) = tag_value(trimmed, "Event")
            && !value.trim().is_empty()
        {
            name = value;
        }
    }

    let mut game = if let Some(fen) = fen {
        Game::from_fen(&fen).map_err(|error| format!("Invalid FEN tag: {error}"))?
    } else {
        Game::new()
    };

    let body = input
        .lines()
        .filter(|line| !line.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join("\n");

    let mut commentary = Vec::new();
    let mut token = String::new();
    let mut comment = String::new();
    let mut in_comment = false;
    let mut variation_depth = 0usize;

    for character in body.chars() {
        if variation_depth > 0 {
            match character {
                '(' => variation_depth += 1,
                ')' => variation_depth = variation_depth.saturating_sub(1),
                _ => {}
            }
            continue;
        }
        if in_comment {
            if character == '}' {
                in_comment = false;
                let text = comment.trim().to_string();
                comment.clear();
                if !text.is_empty()
                    && let Some(last) = game.history().last().cloned()
                {
                    game.annotate_last(Some(text.clone()), last.eval.clone());
                    commentary.push(CommentaryCard {
                        san: last.san.clone(),
                        uci: last.uci.clone(),
                        comment: text,
                        eval: None,
                        candidates: Vec::new(),
                        plan: None,
                        tokens: None,
                        cost: None,
                        elapsed_ms: 0,
                    });
                }
            } else {
                comment.push(character);
            }
            continue;
        }
        match character {
            '{' => {
                apply_token(&token, &mut game)?;
                token.clear();
                in_comment = true;
            }
            '(' => {
                apply_token(&token, &mut game)?;
                token.clear();
                variation_depth = 1;
            }
            ')' => {}
            character if character.is_whitespace() => {
                apply_token(&token, &mut game)?;
                token.clear();
            }
            character => token.push(character),
        }
    }
    apply_token(&token, &mut game)?;

    if game.history().is_empty() {
        return Err("No moves found in the PGN.".to_string());
    }

    if let Some(color) = human_color {
        if color.eq_ignore_ascii_case("black") {
            game.set_human_color(Color::Black);
        } else if color.eq_ignore_ascii_case("white") {
            game.set_human_color(Color::White);
        }
    }

    Ok(Imported {
        game,
        commentary,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_pgn_round_trip() {
        let settings = Settings::default();
        let mut game = Game::new();
        game.apply_san("e4").unwrap();
        game.apply_san("e5").unwrap();
        game.annotate_last(Some("A fine reply.".to_string()), None);
        let pgn = export(&game, &settings);
        assert!(pgn.contains("1. e4 e5 {A fine reply.}"));
        let imported = parse(&pgn).unwrap();
        assert_eq!(imported.game.history().len(), 2);
        assert_eq!(
            imported.game.history()[1].comment.as_deref(),
            Some("A fine reply.")
        );
        assert_eq!(imported.commentary.len(), 1);
    }

    #[test]
    fn imports_numbered_moves_and_comments() {
        let pgn = "[Event \"Test\"]\n\n1. e4 {opening} e5 2. Nf3 Nc6 1-0\n";
        let imported = parse(pgn).unwrap();
        assert_eq!(imported.name, "Test");
        assert_eq!(imported.game.history().len(), 4);
        assert_eq!(imported.commentary.len(), 1);
    }
}
