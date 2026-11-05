//! Local provider credentials.
//!
//! Keys are stored in a user-only JSON file and are never returned to the
//! frontend. They are passed to the OpenCode child process as environment
//! variables. If no key is saved, OpenCode's own authentication is used.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CredentialRecord {
    env: String,
    key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub provider: String,
    pub env_var: Option<String>,
    pub has_key: bool,
}

fn config_dir() -> PathBuf {
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("gambiit");
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("gambiit")
}

fn credentials_path() -> PathBuf {
    config_dir().join("credentials.json")
}

fn read_from(path: &Path) -> BTreeMap<String, CredentialRecord> {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn write_to(path: &Path, records: &BTreeMap<String, CredentialRecord>) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Could not resolve credentials directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;

    let temporary = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(records).map_err(|error| error.to_string())?;
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }

    file.write_all(json.as_bytes())
        .map_err(|error| error.to_string())?;
    file.sync_all().ok();
    fs::rename(&temporary, path).map_err(|error| error.to_string())?;
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }
    Ok(())
}

fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn status_at(path: &Path, provider: &str) -> CredentialStatus {
    let provider = provider.trim();
    let record = read_from(path).get(provider).cloned();
    CredentialStatus {
        provider: provider.to_string(),
        env_var: record.as_ref().map(|record| record.env.clone()),
        has_key: record.is_some(),
    }
}

fn set_at(
    path: &Path,
    provider: &str,
    env_var: &str,
    key: &str,
) -> Result<CredentialStatus, String> {
    let provider = provider.trim();
    let env_var = env_var.trim();
    let key = key.trim();
    if provider.is_empty() {
        return Err("Provider is required.".to_string());
    }
    if !valid_env_name(env_var) {
        return Err(
            "Environment variable must contain only letters, digits and underscores.".to_string(),
        );
    }
    if key.is_empty() {
        return Err("API key is required.".to_string());
    }

    let mut records = read_from(path);
    records.insert(
        provider.to_string(),
        CredentialRecord {
            env: env_var.to_string(),
            key: key.to_string(),
        },
    );
    write_to(path, &records)?;
    Ok(status_at(path, provider))
}

fn delete_at(path: &Path, provider: &str) -> Result<CredentialStatus, String> {
    let provider = provider.trim();
    let mut records = read_from(path);
    records.remove(provider);
    write_to(path, &records)?;
    Ok(status_at(path, provider))
}

fn env_for_provider_at(path: &Path, provider: &str) -> Option<(String, String)> {
    read_from(path)
        .get(provider.trim())
        .map(|record| (record.env.clone(), record.key.clone()))
}

pub fn status(provider: &str) -> CredentialStatus {
    status_at(&credentials_path(), provider)
}

pub fn set(provider: &str, env_var: &str, key: &str) -> Result<CredentialStatus, String> {
    let path = credentials_path();
    ensure_parent(&path)?;
    set_at(&path, provider, env_var, key)
}

pub fn delete(provider: &str) -> Result<CredentialStatus, String> {
    let path = credentials_path();
    ensure_parent(&path)?;
    delete_at(&path, provider)
}

pub fn env_for_provider(provider: &str) -> Option<(String, String)> {
    env_for_provider_at(&credentials_path(), provider)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_environment_names() {
        assert!(valid_env_name("OPENAI_API_KEY"));
        assert!(valid_env_name("OPENCODE_GO_API_KEY"));
        assert!(!valid_env_name(""));
        assert!(!valid_env_name("BAD-NAME"));
        assert!(!valid_env_name("BAD NAME"));
    }

    #[test]
    fn stores_reads_deletes_and_secures_the_file() {
        let path = std::env::temp_dir().join(format!(
            "gambiit-credentials-test-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        let status = set_at(&path, "openai", "OPENAI_API_KEY", "sk-test").unwrap();
        assert!(status.has_key);
        assert_eq!(status.env_var.as_deref(), Some("OPENAI_API_KEY"));
        assert_eq!(
            env_for_provider_at(&path, "openai"),
            Some(("OPENAI_API_KEY".to_string(), "sk-test".to_string()))
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        let status = delete_at(&path, "openai").unwrap();
        assert!(!status.has_key);
        assert!(env_for_provider_at(&path, "openai").is_none());
        let _ = fs::remove_file(&path);
    }
}
