use crate::env_resolver;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{BufRead, BufReader};
use std::time::Duration;

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Serialize)]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<String>,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Deserialize)]
struct OllamaPullResponse {
    status: Option<String>,
    error: Option<String>,
}

pub fn ollama_generate(base_url: &str, model: &str, prompt: &str) -> Result<String, String> {
    if !is_loopback_url(base_url) {
        return Err(
            "Ollama URL loopback olmalı (localhost/127.0.0.1/::1); uzak host'a not içeriği gönderilmez"
                .to_string(),
        );
    }
    let url = format!("{}/api/generate", normalized_base_url(base_url));
    let body = json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
    });

    let mut response = ureq::post(&url)
        .send_json(body)
        .map_err(|err| friendly_ollama_error(base_url, err))?;
    let parsed = response
        .body_mut()
        .read_json::<OllamaGenerateResponse>()
        .map_err(|err| format!("failed to read Ollama response: {err}"))?;

    Ok(parsed.response)
}

pub fn ollama_available(base_url: &str) -> bool {
    let url = format!("{}/api/tags", normalized_base_url(base_url));
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build();
    let agent = config.new_agent();
    agent.get(&url).call().is_ok()
}

pub fn ollama_status(base_url: Option<String>) -> OllamaStatus {
    let installed = env_resolver::login_command("/bin/zsh")
        .args(["-lc", "command -v ollama"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    let base_url = base_url.unwrap_or_else(default_ollama_base_url);
    let url = format!("{}/api/tags", normalized_base_url(&base_url));
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build();
    let agent = config.new_agent();

    let models = match agent.get(&url).call() {
        Ok(mut response) => match response.body_mut().read_json::<OllamaTagsResponse>() {
            Ok(tags) => tags.models.into_iter().map(|model| model.name).collect(),
            Err(_) => {
                return OllamaStatus {
                    installed,
                    running: false,
                    models: Vec::new(),
                };
            }
        },
        Err(_) => {
            return OllamaStatus {
                installed,
                running: false,
                models: Vec::new(),
            };
        }
    };

    OllamaStatus {
        installed,
        running: true,
        models,
    }
}

pub fn ollama_pull<F>(model: &str, base_url: Option<String>, mut on_status: F) -> Result<(), String>
where
    F: FnMut(String) -> Result<(), String>,
{
    let base_url = base_url.unwrap_or_else(default_ollama_base_url);
    let url = format!("{}/api/pull", normalized_base_url(&base_url));
    let body = json!({
        "name": model,
        "stream": true,
    });
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60 * 60)))
        .build();
    let agent = config.new_agent();
    let mut response = agent
        .post(&url)
        .send_json(body)
        .map_err(|err| friendly_ollama_error(&base_url, err))?;

    let reader = BufReader::new(response.body_mut().as_reader());
    for line in reader.lines() {
        let line = line.map_err(|err| format!("failed to read Ollama pull stream: {err}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let status = parse_pull_status_line(&line);
        on_status(status)?;
    }

    Ok(())
}

pub fn parse_ollama_tags_json(json_text: &str) -> Vec<String> {
    serde_json::from_str::<OllamaTagsResponse>(json_text)
        .map(|tags| tags.models.into_iter().map(|model| model.name).collect())
        .unwrap_or_default()
}

fn parse_pull_status_line(line: &str) -> String {
    match serde_json::from_str::<OllamaPullResponse>(line) {
        Ok(response) => response
            .error
            .map(|error| format!("error: {error}"))
            .or(response.status)
            .unwrap_or_else(|| line.to_string()),
        Err(_) => line.to_string(),
    }
}

fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
}

fn normalized_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

/// GÜVENLİK (codex #3): Ollama URL'i SADECE loopback olmalı. Aksi halde not-context'i
/// uzak bir "ollama"ya gönderip veri sızdırma riski. Default http://localhost:11434 geçer.
pub fn is_loopback_url(base_url: &str) -> bool {
    let url = normalized_base_url(base_url);
    let after_scheme = url.split("://").nth(1).unwrap_or(url.as_str());
    let authority = after_scheme.split('/').next().unwrap_or("");
    // userinfo bypass'ını engelle: http://localhost@evil.com → '@' varsa reddet (codex).
    if authority.contains('@') {
        return false;
    }
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or("") // [::1]:port → ::1
    } else {
        authority.rsplit_once(':').map(|(h, _)| h).unwrap_or(authority)
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn friendly_ollama_error(base_url: &str, err: ureq::Error) -> String {
    format!(
        "Ollama is not reachable at {}. Start Ollama and check the configured URL. ({err})",
        normalized_base_url(base_url)
    )
}
