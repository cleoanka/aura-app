//! BYOK (bring-your-own-key) Anthropic API key storage.
//!
//! The key is kept in a single 0600 file at `~/.aura/anthropic_api_key` — the
//! SAME location the `aura` CLI reads — so one key drives both the desktop app
//! and the terminal CLI. The app only injects it into child processes when the
//! user has enabled BYOK in Settings (`api_key_enabled`); otherwise the existing
//! subscription / OAuth path is used unchanged. The key value is never logged.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const KEY_FILE: &str = "anthropic_api_key";

/// Mirror the CLI's home resolution: `$AURA_RUNS_DIR_HOME` if set, else `~/.aura`.
fn aura_home() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("AURA_RUNS_DIR_HOME") {
        if !custom.trim().is_empty() {
            return Some(PathBuf::from(custom));
        }
    }
    dirs::home_dir().map(|home| home.join(".aura"))
}

pub fn key_path() -> Option<PathBuf> {
    aura_home().map(|dir| dir.join(KEY_FILE))
}

/// Parse the key-file contents: the first non-empty, trimmed line. Robust to a
/// stray trailing line or accidental newline (which `trim()` alone would keep,
/// corrupting the key). Pure → unit-testable.
fn parse_key_file(contents: &str) -> Option<String> {
    contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

/// The stored key; `None` if absent or empty.
pub fn read_key() -> Option<String> {
    let raw = fs::read_to_string(key_path()?).ok()?;
    parse_key_file(&raw)
}

/// Trim + validate a key WITHOUT touching disk (pure → unit-testable). A real key
/// is a single token; rejecting internal whitespace catches the common paste error
/// (multi-line paste, `Bearer sk-…`, trailing junk) before it's stored.
fn validate_key(key: &str) -> Result<&str, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key is empty".to_string());
    }
    if key.split_whitespace().count() != 1 {
        return Err("API key must be a single token (no spaces or newlines)".to_string());
    }
    Ok(key)
}

pub fn write_key(key: &str) -> Result<(), String> {
    let key = validate_key(key)?;
    let dir = aura_home().ok_or_else(|| "could not resolve home directory".to_string())?;
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create {}: {err}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }
    write_private(&dir.join(KEY_FILE), key)
}

pub fn clear_key() -> Result<(), String> {
    let Some(path) = key_path() else {
        return Ok(());
    };
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove API key: {err}")),
    }
}

#[derive(Serialize)]
pub struct ApiKeyStatus {
    pub present: bool,
    /// A masked preview (e.g. `sk-…aB3d`) — never the full key.
    pub masked: Option<String>,
}

pub fn status() -> ApiKeyStatus {
    match read_key() {
        Some(key) => ApiKeyStatus {
            present: true,
            masked: Some(mask(&key)),
        },
        None => ApiKeyStatus {
            present: false,
            masked: None,
        },
    }
}

/// The key to inject into spawned children — only when the user enabled BYOK
/// AND a key is stored. `None` otherwise (default subscription / OAuth path).
pub fn child_anthropic_key() -> Option<String> {
    if crate::settings::load().api_key_enabled {
        read_key()
    } else {
        None
    }
}

fn mask(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    let n = chars.len();
    if n <= 8 {
        return "•".repeat(n.max(1));
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[n - 4..].iter().collect();
    format!("{head}…{tail}")
}

fn write_private(path: &Path, content: &str) -> Result<(), String> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| format!("failed to write API key: {err}"))?;
    file.write_all(content.as_bytes())
        .map_err(|err| format!("failed to write API key: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_hides_the_middle() {
        assert_eq!(mask("sk-ant-api03-abcd1234"), "sk-…1234");
        assert_eq!(mask("short"), "•••••");
    }

    #[test]
    fn mask_never_returns_the_full_key() {
        let key = "sk-ant-secret-value-1234";
        assert!(!mask(key).contains("secret"));
    }

    #[test]
    fn validate_key_accepts_a_clean_single_token() {
        assert_eq!(validate_key("  sk-ant-api03-abcd  ").unwrap(), "sk-ant-api03-abcd");
    }

    #[test]
    fn validate_key_rejects_empty_and_whitespace_only() {
        assert!(validate_key("").is_err());
        assert!(validate_key("   ").is_err());
    }

    #[test]
    fn validate_key_rejects_internal_whitespace() {
        assert!(validate_key("sk-ant foo").is_err()); // accidental space
        assert!(validate_key("Bearer sk-ant-xyz").is_err()); // pasted prefix
        assert!(validate_key("sk-ant-1\nsk-ant-2").is_err()); // multi-line paste
    }

    #[test]
    fn parse_key_file_takes_first_nonempty_line() {
        assert_eq!(parse_key_file("sk-ant-x\n").as_deref(), Some("sk-ant-x"));
        assert_eq!(parse_key_file("\n  sk-ant-y  \n# note\n").as_deref(), Some("sk-ant-y"));
        assert_eq!(parse_key_file("\n\n"), None);
        assert_eq!(parse_key_file(""), None);
    }
}
