#[test]
#[ignore]
fn ollama_generate_live() -> Result<(), String> {
    let base_url = std::env::var("AURA_TEST_OLLAMA_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());
    let model = std::env::var("AURA_TEST_OLLAMA_MODEL")
        .map_err(|_| "set AURA_TEST_OLLAMA_MODEL for this ignored test".to_string())?;

    if !app_lib::lane0::ollama_available(&base_url) {
        return Err(format!("Ollama is not available at {base_url}"));
    }

    let response = app_lib::lane0::ollama_generate(&base_url, &model, "Say ok.")?;

    assert!(!response.trim().is_empty());
    Ok(())
}
