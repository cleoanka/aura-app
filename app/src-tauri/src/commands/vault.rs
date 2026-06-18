use crate::db::NoteRef;
use crate::indexer::Indexer;
use crate::search::SearchHit;
use crate::settings::{self, Settings};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn pick_vault_folder(app: AppHandle) -> Result<Option<String>, String> {
    // Dialog'u callback + channel ile aç: senkron komut + blocking_pick_folder
    // ana thread'i kilitleyip çökertiyordu. async komut + non-blocking callback güvenli.
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let Some(folder) = rx.recv().map_err(|err| err.to_string())? else {
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

#[tauri::command]
pub fn read_note(path: String) -> Result<String, String> {
    let settings = settings::load();
    let path = resolve_note_path(&path, &settings)?;
    fs::read_to_string(&path).map_err(|err| format!("failed to read {}: {err}", path.display()))
}

#[tauri::command]
pub fn write_note(path: String, content: String) -> Result<(), String> {
    let settings = settings::load();
    let path = resolve_note_path_for_write(&path, &settings)?;
    fs::write(&path, content).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

/// AI çıktısını (plan/cevap/inceleme) projenin "AURA/" klasörüne not olarak kaydeder.
/// Plan→eylem köprüsü: çıktı artık ölü-uçta kalmaz. Kaydedilen dosyanın yolunu döner.
#[tauri::command]
pub fn save_note(kind: String, content: String) -> Result<String, String> {
    let settings = settings::load();
    let root = settings
        .vault_roots
        .first()
        .ok_or_else(|| "no project folder selected".to_string())?;
    let dir = PathBuf::from(root).join("AURA");
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create {}: {err}", dir.display()))?;
    // Milisaniye hassasiyeti: aynı saniyede iki kayıt birbirini EZMESİN.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let safe_kind: String = kind
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let safe_kind = if safe_kind.is_empty() {
        "aura".to_string()
    } else {
        safe_kind
    };
    let path = dir.join(format!("{safe_kind}-{stamp}.md"));
    fs::write(&path, content).map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn resolve_note_path(path: &str, settings: &Settings) -> Result<PathBuf, String> {
    let requested = PathBuf::from(path);
    let canonical = requested
        .canonicalize()
        .map_err(|err| format!("failed to resolve note path: {err}"))?;

    if is_under_vault_root(&canonical, settings)? {
        Ok(canonical)
    } else {
        Err("note path is outside configured vault roots".to_string())
    }
}

pub fn resolve_note_path_for_write(path: &str, settings: &Settings) -> Result<PathBuf, String> {
    let requested = PathBuf::from(path);
    if requested.exists() {
        return resolve_note_path(path, settings);
    }

    let parent = requested
        .parent()
        .ok_or_else(|| "note path has no parent directory".to_string())?;
    let filename = requested
        .file_name()
        .ok_or_else(|| "note path has no file name".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|err| format!("failed to resolve note parent path: {err}"))?;

    if !is_under_vault_root(&canonical_parent, settings)? {
        return Err("note path is outside configured vault roots".to_string());
    }

    Ok(canonical_parent.join(filename))
}

fn is_under_vault_root(path: &Path, settings: &Settings) -> Result<bool, String> {
    if settings.vault_roots.is_empty() {
        return Ok(false);
    }

    for root in &settings.vault_roots {
        let root = PathBuf::from(root);
        let Ok(root) = root.canonicalize() else {
            continue;
        };
        if path.starts_with(root) {
            return Ok(true);
        }
    }

    Ok(false)
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
