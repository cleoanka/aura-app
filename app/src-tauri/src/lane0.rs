use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

pub fn ollama_generate(base_url: &str, model: &str, prompt: &str) -> Result<String, String> {
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

fn normalized_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn friendly_ollama_error(base_url: &str, err: ureq::Error) -> String {
    format!(
        "Ollama is not reachable at {}. Start Ollama and check the configured URL. ({err})",
        normalized_base_url(base_url)
    )
}
