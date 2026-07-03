use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

static LOGIN_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn login_env() -> &'static HashMap<String, String> {
    LOGIN_ENV.get_or_init(capture_login_env)
}

pub fn login_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command.env_clear().envs(login_env());
    command
}

/// Bozuk dotfile (sonsuz döngü, ağ bekleyen nvm vs.) tüm uygulamayı asamaz:
/// login-shell env yakalamaya sert üst sınır (audit C9).
const LOGIN_ENV_TIMEOUT: Duration = Duration::from_secs(10);

fn capture_login_env() -> HashMap<String, String> {
    let output = login_env_output_with_timeout();

    let mut env = match output {
        Some(stdout) => parse_env_output(&stdout),
        None => std::env::vars().collect(),
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

/// `/bin/zsh -lc env -0` çıktısını TIMEOUT ile al: bekleme poll'lu, süre aşımında
/// süreç öldürülür ve None döner (çağıran mevcut sürecin env'ine düşer).
/// Okuyucu sonucu channel'la alınır (codex: dotfile'ın stdout'u miras bırakan bir
/// arka-plan süreci join()'i child çıktıktan sonra bile süresiz asabilirdi).
fn login_env_output_with_timeout() -> Option<Vec<u8>> {
    let mut child = Command::new("/bin/zsh")
        .args(["-lc", "env -0"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // stdout'u ayrı thread'de oku: pipe dolarsa child bloklanmasın (deadlock önlemi).
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });

    let started = Instant::now();
    let remaining = |started: Instant| LOGIN_ENV_TIMEOUT.saturating_sub(started.elapsed());
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                // Kalan bütçe içinde çıktıyı bekle; gelmezse (stdout'u tutan torun
                // süreç) None → mevcut env'e düş. Thread arkada kendi kendine biter.
                return rx.recv_timeout(remaining(started).max(Duration::from_millis(50))).ok();
            }
            Ok(Some(_)) => return None,
            Ok(None) if started.elapsed() > LOGIN_ENV_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return None,
        }
    }
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
