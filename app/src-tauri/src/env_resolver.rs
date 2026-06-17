use std::collections::HashMap;
use std::process::Command;
use std::sync::OnceLock;

static LOGIN_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn login_env() -> &'static HashMap<String, String> {
    LOGIN_ENV.get_or_init(capture_login_env)
}

pub fn login_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command.env_clear().envs(login_env());
    command
}

fn capture_login_env() -> HashMap<String, String> {
    let output = Command::new("/bin/zsh").args(["-lc", "env -0"]).output();

    let mut env = match output {
        Ok(output) if output.status.success() => parse_env_output(&output.stdout),
        _ => std::env::vars().collect(),
    };

    // HOME garanti olsun.
    let home = env
        .get("HOME")
        .cloned()
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_default();
    if !home.is_empty() {
        env.entry("HOME".to_string()).or_insert_with(|| home.clone());
    }

    // KRİTİK: GUI/Launchpad'den açılınca login-shell PATH'i kullanıcı CLI dizinlerini
    // (ör. ~/.local/bin, ~/.npm-global/bin) içermeyebilir → aura/claude/gemini/codex
    // "command not found". PATH'i bilinen dizinlerle deterministik güçlendir.
    augment_path(&mut env, &home);
    env
}

fn augment_path(env: &mut HashMap<String, String>, home: &str) {
    let mut dirs: Vec<String> = Vec::new();
    if !home.is_empty() {
        for sub in [".local/bin", ".npm-global/bin", "bin", ".cargo/bin", ".deno/bin"] {
            dirs.push(format!("{home}/{sub}"));
        }
    }
    for d in [
        "/opt/homebrew/bin",
        "/opt/homebrew/sbin",
        "/usr/local/bin",
        "/usr/local/sbin",
        "/usr/bin",
        "/bin",
        "/usr/sbin",
        "/sbin",
    ] {
        dirs.push(d.to_string());
    }
    if let Some(existing) = env.get("PATH") {
        for d in existing.split(':') {
            if !d.is_empty() {
                dirs.push(d.to_string());
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let merged: Vec<String> = dirs
        .into_iter()
        .filter(|d| seen.insert(d.clone()))
        .collect();
    env.insert("PATH".to_string(), merged.join(":"));
}

fn parse_env_output(stdout: &[u8]) -> HashMap<String, String> {
    stdout
        .split(|byte| *byte == b'\0')
        .filter_map(|entry| {
            let separator = entry.iter().position(|byte| *byte == b'=')?;
            let key = String::from_utf8_lossy(&entry[..separator]).into_owned();
            let value = String::from_utf8_lossy(&entry[separator + 1..]).into_owned();

            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}
