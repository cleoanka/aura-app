use crate::db::NoteRef;
use crate::indexer::Indexer;
use crate::search::SearchHit;
use crate::settings;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub fn pick_vault_folder(app: AppHandle) -> Result<Option<String>, String> {
    let Some(folder) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let path = picked_folder_to_string(folder)?;

    let mut settings = settings::load();
    if !settings.vault_roots.iter().any(|root| root == &path) {
        settings.vault_roots.push(path.clone());
        settings::save(&settings)?;
    }

    Ok(Some(path))
}

#[tauri::command]
pub fn list_notes(indexer: State<'_, Mutex<Indexer>>) -> Result<Vec<NoteRef>, String> {
    let indexer = indexer.lock().map_err(|err| err.to_string())?;
    crate::db::list_notes(indexer.conn()).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn search_hybrid(
    indexer: State<'_, Mutex<Indexer>>,
    query: String,
    k: u32,
) -> Result<Vec<SearchHit>, String> {
    let indexer = indexer.lock().map_err(|err| err.to_string())?;
    indexer.search_hybrid(&query, k as usize)
}

fn picked_folder_to_string(folder: impl Serialize) -> Result<String, String> {
    let value = serde_json::to_value(folder)
        .map_err(|err| format!("failed to serialize selected folder: {err}"))?;
    folder_value_to_string(&value).ok_or_else(|| "selected folder is not a local path".to_string())
}

fn folder_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(path) => Some(path.clone()),
        serde_json::Value::Object(object) => object
            .get("path")
            .or_else(|| object.get("Path"))
            .or_else(|| object.get("filePath"))
            .and_then(folder_value_to_string),
        _ => None,
    }
}
