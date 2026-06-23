use crate::apikey::{self, ApiKeyStatus};
use crate::settings;

#[tauri::command]
pub fn api_key_status() -> ApiKeyStatus {
    apikey::status()
}

/// Store a BYOK Anthropic API key (0600 file shared with the `aura` CLI) and
/// turn BYOK on. The key value is never logged.
#[tauri::command]
pub fn set_api_key(key: String) -> Result<ApiKeyStatus, String> {
    apikey::write_key(&key)?;
    let mut current = settings::load();
    if !current.api_key_enabled {
        current.api_key_enabled = true;
        settings::save(&current)?;
    }
    Ok(apikey::status())
}

/// Remove the stored key and turn BYOK off.
#[tauri::command]
pub fn clear_api_key() -> Result<ApiKeyStatus, String> {
    apikey::clear_key()?;
    let mut current = settings::load();
    if current.api_key_enabled {
        current.api_key_enabled = false;
        settings::save(&current)?;
    }
    Ok(apikey::status())
}
