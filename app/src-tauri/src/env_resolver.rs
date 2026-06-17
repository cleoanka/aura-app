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

    match output {
        Ok(output) if output.status.success() => parse_env_output(&output.stdout),
        _ => std::env::vars().collect(),
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
