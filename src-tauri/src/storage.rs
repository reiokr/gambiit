//! Local game library, resume data and PGN exports.

use crate::{ChatMessage, CommentaryCard, Settings};
use gambiit_core::MoveRecord;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedGame {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub human_color: String,
    pub initial_fen: String,
    pub moves: Vec<MoveRecord>,
    pub commentary: Vec<CommentaryCard>,
    #[serde(default)]
    pub chat: Vec<ChatMessage>,
    pub settings: Settings,
    pub result: String,
    #[serde(default = "default_termination")]
    pub termination: String,
}

fn default_termination() -> String {
    "ongoing".to_string()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSummary {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
    pub human_color: String,
    pub model: String,
    pub move_count: usize,
    pub result: String,
}

fn data_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("gambiit");
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".local").join("share").join("gambiit")
}

pub fn games_dir() -> PathBuf {
    data_dir().join("games")
}

pub fn pieces_dir() -> PathBuf {
    data_dir().join("pieces")
}

fn exports_dir() -> PathBuf {
    data_dir().join("exports")
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("g{:x}", nanos)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| error.to_string())
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve output directory.".to_string())?;
    ensure_dir(parent)?;
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
    }
    file.write_all(contents.as_bytes())
        .map_err(|error| error.to_string())?;
    file.sync_all().ok();
    fs::rename(&temporary, path).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn save(game: &SavedGame) -> Result<(), String> {
    if !valid_id(&game.id) {
        return Err("Invalid game id.".to_string());
    }
    let json = serde_json::to_string_pretty(game).map_err(|error| error.to_string())?;
    write_atomic(&games_dir().join(format!("{}.json", game.id)), &json)
}

pub fn load(id: &str) -> Result<SavedGame, String> {
    if !valid_id(id) {
        return Err("Invalid game id.".to_string());
    }
    let path = games_dir().join(format!("{id}.json"));
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&raw).map_err(|error| format!("Could not read saved game: {error}"))
}

pub fn list() -> Result<Vec<GameSummary>, String> {
    let dir = games_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(dir).map_err(|error| error.to_string())?;
    let mut games = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(game) = serde_json::from_str::<SavedGame>(&raw) else {
            continue;
        };
        games.push(GameSummary {
            id: game.id,
            name: game.name,
            updated_at: game.updated_at,
            human_color: game.human_color,
            model: game.settings.model,
            move_count: game.moves.len(),
            result: game.result,
        });
    }
    games.sort_by_key(|game| std::cmp::Reverse(game.updated_at));
    Ok(games)
}

pub fn delete(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("Invalid game id.".to_string());
    }
    let path = games_dir().join(format!("{id}.json"));
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn rename(id: &str, name: &str) -> Result<GameSummary, String> {
    let mut game = load(id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Game name cannot be empty.".to_string());
    }
    game.name = name.chars().take(80).collect();
    game.updated_at = now();
    save(&game)?;
    Ok(GameSummary {
        id: game.id,
        name: game.name,
        updated_at: game.updated_at,
        human_color: game.human_color,
        model: game.settings.model,
        move_count: game.moves.len(),
        result: game.result,
    })
}

fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('-').to_string();
    if cleaned.is_empty() {
        "game".to_string()
    } else {
        cleaned.chars().take(80).collect()
    }
}

pub fn save_pgn(name: &str, pgn: &str) -> Result<String, String> {
    let filename = format!("{}.pgn", sanitize_name(name));
    let path = exports_dir().join(filename);
    write_atomic(&path, pgn)?;
    Ok(path.display().to_string())
}
