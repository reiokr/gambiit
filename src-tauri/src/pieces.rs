//! Custom piece-set discovery and loading.
//!
//! A set is a folder under `~/.local/share/gambiit/pieces/` containing up to
//! twelve files named `wK`, `wQ`, `wR`, `wB`, `wN`, `wP`, `bK`, `bQ`, `bR`,
//! `bB`, `bN` and `bP` with an `.svg`, `.png`, `.webp`, `.jpg` or `.jpeg`
//! extension. Missing pieces fall back to the built-in set.

use serde::Serialize;
use std::{fs, path::Path};

const BUILT_IN_SETS: [(&str, &str); 5] = [
    ("cburnett", "Cburnett"),
    ("livius", "Livius"),
    ("meridian", "Meridian"),
    ("meridian-shaded", "Meridian Shaded"),
    ("letter", "Letter"),
];
const CODES: [&str; 12] = [
    "wK", "wQ", "wR", "wB", "wN", "wP", "bK", "bQ", "bR", "bB", "bN", "bP",
];
const EXTENSIONS: [(&str, &str); 5] = [
    ("svg", "image/svg+xml"),
    ("png", "image/png"),
    ("webp", "image/webp"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
];
const MAX_PIECE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PieceSetSummary {
    pub id: String,
    pub name: String,
    pub built_in: bool,
    pub pieces: usize,
    pub preview: Option<String>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn display_name(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn encode_base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(ALPHABET[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(third & 0b111111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn find_piece(directory: &Path, code: &str) -> Option<(String, &'static str)> {
    for (extension, mime) in EXTENSIONS {
        let path = directory.join(format!("{code}.{extension}"));
        if path.is_file() {
            return Some((path.display().to_string(), mime));
        }
    }
    None
}

fn file_data_url(path: &str, mime: &str, max_bytes: u64) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    Some(format!("data:{mime};base64,{}", encode_base64(&bytes)))
}

fn count_pieces(directory: &Path) -> usize {
    CODES
        .iter()
        .filter(|code| find_piece(directory, code).is_some())
        .count()
}

fn preview_for(directory: &Path) -> Option<String> {
    for code in [
        "wN", "bN", "wK", "bK", "wQ", "bQ", "wR", "bR", "wB", "bB", "wP", "bP",
    ] {
        if let Some((path, mime)) = find_piece(directory, code)
            && let Some(url) = file_data_url(&path, mime, 256 * 1024)
        {
            return Some(url);
        }
    }
    None
}

fn is_built_in(id: &str) -> bool {
    BUILT_IN_SETS.iter().any(|(set_id, _)| *set_id == id)
}

pub fn list() -> Result<Vec<PieceSetSummary>, String> {
    list_at(&crate::storage::pieces_dir())
}

fn list_at(root: &Path) -> Result<Vec<PieceSetSummary>, String> {
    let mut sets: Vec<PieceSetSummary> = BUILT_IN_SETS
        .iter()
        .map(|(id, name)| PieceSetSummary {
            id: (*id).to_string(),
            name: (*name).to_string(),
            built_in: true,
            pieces: 12,
            preview: None,
        })
        .collect();
    if !root.exists() {
        return Ok(sets);
    }
    let entries = fs::read_dir(root).map_err(|error| error.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !valid_id(id) {
            continue;
        }
        let pieces = count_pieces(&path);
        if pieces == 0 {
            continue;
        }
        sets.push(PieceSetSummary {
            id: id.to_string(),
            name: display_name(id),
            built_in: false,
            pieces,
            preview: preview_for(&path),
        });
    }
    let built_in_count = BUILT_IN_SETS.len();
    sets[built_in_count..].sort_by(|left, right| left.name.cmp(&right.name));
    Ok(sets)
}

pub fn load(id: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    load_at(&crate::storage::pieces_dir(), id)
}

fn load_at(root: &Path, id: &str) -> Result<std::collections::BTreeMap<String, String>, String> {
    let mut urls = std::collections::BTreeMap::new();
    if is_built_in(id) {
        return Ok(urls);
    }
    if !valid_id(id) {
        return Err("Invalid piece set id.".to_string());
    }
    let directory = root.join(id);
    if !directory.is_dir() {
        return Err(format!("Piece set '{id}' was not found."));
    }
    for code in CODES {
        if let Some((path, mime)) = find_piece(&directory, code)
            && let Some(url) = file_data_url(&path, mime, MAX_PIECE_BYTES)
        {
            urls.insert(code.to_string(), url);
        }
    }
    Ok(urls)
}

pub fn open_folder() -> Result<(), String> {
    let directory = crate::storage::pieces_dir();
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let mut command = if cfg!(target_os = "windows") {
        let mut command = std::process::Command::new("explorer");
        command.arg(&directory);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = std::process::Command::new("open");
        command.arg(&directory);
        command
    } else {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(&directory);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_base64_with_padding() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"hello"), "aGVsbG8=");
    }

    #[test]
    fn validates_set_ids() {
        assert!(valid_id("my-set_1"));
        assert!(!valid_id(""));
        assert!(!valid_id("../escape"));
        assert!(!valid_id("with space"));
        assert!(!valid_id(&"x".repeat(65)));
    }

    #[test]
    fn makes_readable_names() {
        assert_eq!(display_name("my-cool_set"), "My Cool Set");
        assert_eq!(display_name("classic"), "Classic");
    }

    #[test]
    fn lists_and_loads_a_partial_set() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "gambiit-pieces-test-{}-{nanos}",
            std::process::id()
        ));
        let set = root.join("my-set");
        fs::create_dir_all(&set).unwrap();
        fs::write(set.join("wN.svg"), b"<svg/>").unwrap();
        fs::write(set.join("wK.png"), b"png").unwrap();

        let sets = list_at(&root).unwrap();
        assert_eq!(sets.len(), 6);
        assert_eq!(sets[0].id, "cburnett");
        assert!(sets[0].built_in);
        assert!(sets.iter().any(|set| set.id == "livius" && set.built_in));
        let custom = sets.iter().find(|set| set.id == "my-set").unwrap();
        assert_eq!(custom.name, "My Set");
        assert_eq!(custom.pieces, 2);
        assert!(
            custom
                .preview
                .as_deref()
                .is_some_and(|url| url.starts_with("data:image/svg+xml;base64,"))
        );

        let loaded = load_at(&root, "my-set").unwrap();
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains_key("wN"));
        assert!(loaded.contains_key("wK"));
        assert!(!loaded.contains_key("wQ"));

        assert!(load_at(&root, "../escape").is_err());
        assert!(load_at(&root, "missing").is_err());
        let _ = fs::remove_dir_all(&root);
    }
}
