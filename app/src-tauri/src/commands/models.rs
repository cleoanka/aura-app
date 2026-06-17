use crate::{embed, lane0};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{ipc::Channel, AppHandle};

const EMBEDDING_MODEL: &str = "intfloat/multilingual-e5-small";

#[derive(Serialize)]
pub struct EmbeddingStatus {
    pub backend: String,
    pub model: String,
    pub ready: bool,
    pub downloading: bool,
    pub device: String,
    pub cache_path: Option<String>,
}

#[tauri::command]
pub fn embedding_status() -> EmbeddingStatus {
    let cache_path = find_embedding_cache_path();
    EmbeddingStatus {
        backend: embedding_backend().to_string(),
        model: EMBEDDING_MODEL.to_string(),
        ready: cache_path.is_some(),
        downloading: false,
        device: "cpu".to_string(),
        cache_path: cache_path.map(|path| path.to_string_lossy().into_owned()),
    }
}

#[tauri::command]
pub async fn prepare_embedding_model(
    _app: AppHandle,
    on_event: Channel<String>,
) -> Result<EmbeddingStatus, String> {
    on_event
        .send("downloading".to_string())
        .map_err(|err| format!("failed to send embedding event: {err}"))?;

    let result = tokio::task::spawn_blocking(move || {
        // AÇIKÇA indir (default_embedder indirme yapmaz). candle yoksa hata döner.
        embed::force_prepare_candle()?;
        let status = embedding_status();
        if status.ready {
            Ok(status)
        } else {
            Err("indirme sonrası model hazır görünmüyor".to_string())
        }
    })
    .await
    .map_err(|err| format!("embedding preparation task failed: {err}"))?;

    match result {
        Ok(status) => {
            on_event
                .send("ready".to_string())
                .map_err(|err| format!("failed to send embedding event: {err}"))?;
            Ok(status)
        }
        Err(err) => {
            let _ = on_event.send(format!("error: {err}"));
            Err(err)
        }
    }
}

#[tauri::command]
pub fn ollama_status(base_url: Option<String>) -> lane0::OllamaStatus {
    lane0::ollama_status(base_url)
}

#[tauri::command]
pub async fn ollama_pull(
    _app: AppHandle,
    model: String,
    base_url: Option<String>,
    on_event: Channel<String>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        lane0::ollama_pull(&model, base_url, |status| {
            on_event
                .send(status)
                .map_err(|err| format!("failed to send Ollama pull event: {err}"))
        })
    })
    .await
    .map_err(|err| format!("Ollama pull task failed: {err}"))?
}

fn embedding_backend() -> &'static str {
    if cfg!(feature = "candle") {
        "candle"
    } else {
        "stub"
    }
}

fn find_embedding_cache_path() -> Option<PathBuf> {
    let hub = dirs::home_dir()?
        .join(".cache")
        .join("huggingface")
        .join("hub");
    let model_dir_name = format!("models--{}", EMBEDDING_MODEL.replace('/', "--"));
    let preferred = hub.join(&model_dir_name);
    if let Some(path) = find_model_files_dir(&preferred, 0) {
        return Some(path);
    }
    find_model_files_dir(&hub, 0)
}

fn find_model_files_dir(path: &Path, depth: usize) -> Option<PathBuf> {
    if depth > 5 || !path.is_dir() {
        return None;
    }

    let path_text = path.to_string_lossy();
    let model_marker = EMBEDDING_MODEL.replace('/', "--");
    if path_text.contains(EMBEDDING_MODEL) || path_text.contains(&model_marker) {
        let has_tokenizer = path.join("tokenizer.json").is_file();
        let has_weights =
            path.join("model.safetensors").is_file() || path.join("pytorch_model.bin").is_file();
        if has_tokenizer && has_weights {
            return Some(path.to_path_buf());
        }
    }

    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            if let Some(found) = find_model_files_dir(&child, depth + 1) {
                return Some(found);
            }
        }
    }

    None
}
