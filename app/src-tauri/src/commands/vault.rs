use crate::db::NoteRef;
use crate::indexer::Indexer;
use crate::search::SearchHit;
use crate::settings::{self, Settings};
use crate::ReadDb;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn pick_vault_folder(app: AppHandle) -> Result<Option<String>, String> {
    // Dialog'u callback + oneshot ile aç: senkron komut + blocking_pick_folder ana thread'i
    // kilitleyip çökertiyordu; mpsc recv() de dialog açıkken bir runtime worker'ını park
    // ediyordu (audit S21). oneshot + await hiçbir thread'i bloklamaz.
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let Some(folder) = rx.await.map_err(|err| err.to_string())? else {
        return Ok(None);
    };
    let path = picked_folder_to_string(folder)?;

    // Workspace semantiği: seçilen klasör AKTİF workspace olur (MRU başı).
    let mut settings = settings::load();
    if settings::promote_root(&mut settings.vault_roots, &path) {
        settings::save(&settings)?;
    }

    Ok(Some(path))
}

/// Aktif workspace + son kullanılanlar (UI switcher'ı için).
#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceInfo {
    pub active: Option<String>,
    pub recents: Vec<String>,
}

#[tauri::command]
pub fn get_workspace() -> WorkspaceInfo {
    let settings = settings::load();
    WorkspaceInfo {
        active: settings.active_root().map(str::to_string),
        recents: settings.vault_roots.clone(),
    }
}

/// Son-kullanılanlardan birini aktif workspace yapar. Klasör hâlâ mevcut olmalı.
#[tauri::command]
pub fn set_active_workspace(path: String) -> Result<WorkspaceInfo, String> {
    if !Path::new(&path).is_dir() {
        return Err(format!("workspace folder no longer exists: {path}"));
    }
    let mut settings = settings::load();
    if settings::promote_root(&mut settings.vault_roots, &path) {
        settings::save(&settings)?;
    }
    Ok(get_workspace())
}

/// Workspace'i listeden çıkarır ve indeksteki tüm izlerini siler (repo mantığı:
/// unutulan repo'nun verisi arama/graph'ta hayalet bırakmaz, DB şişmez).
/// PERF (audit C7): Indexer kilidi beklenebilir → spawn_blocking, ana thread donmaz.
#[tauri::command]
pub async fn forget_workspace(app: AppHandle, path: String) -> Result<WorkspaceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // Sıra: ÖNCE DB temizliği, commit BAŞARILIYSA settings (codex: ters sıra,
        // purge düşerse workspace listeden gitmiş ama izleri DB'de kalmış bırakıyordu).
        {
            let state = app.state::<Mutex<Indexer>>();
            let indexer = state.lock().map_err(|err| err.to_string())?;
            let tx = crate::indexer::TxGuard::begin(indexer.conn())?;
            crate::db::delete_notes_under(indexer.conn(), &path).map_err(|err| err.to_string())?;
            tx.commit()?;
        }

        let mut settings = settings::load();
        let before = settings.vault_roots.len();
        settings.vault_roots.retain(|root| root != &path);
        if settings.vault_roots.len() != before {
            settings::save(&settings)?;
        }

        Ok(get_workspace())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn list_notes(read: State<'_, ReadDb>) -> Result<Vec<NoteRef>, String> {
    // Ayrı read connection → indeksleme sürerken dosya listesi DONMAZ (codex #2 güvenli dilim).
    let settings = settings::load();
    let conn = read.0.lock().map_err(|err| err.to_string())?;
    // Workspace semantiği: yalnız AKTİF kökün notları (eski repo'lar listede görünmez).
    crate::db::list_notes_under(&conn, settings.active_root()).map_err(|err| err.to_string())
}

// PERF (audit C7/C21): sync komut ana thread'de Indexer kilidini bekliyordu →
// indeksleme sırasında arama tüm UI'ı donduruyordu. async + spawn_blocking.
#[tauri::command]
pub async fn search_hybrid(
    app: AppHandle,
    query: String,
    k: u32,
) -> Result<Vec<SearchHit>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let settings = settings::load();
        let state = app.state::<Mutex<Indexer>>();
        let indexer = state.lock().map_err(|err| err.to_string())?;
        // GÜVENLİK (codex #7): istemci-kontrollü k'yi sınırla → dev allocation olmasın.
        indexer.search_hybrid_in(&query, (k as usize).clamp(1, 50), settings.active_root())
    })
    .await
    .map_err(|err| err.to_string())?
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
    atomic_write(&path, &content)
}

/// Atomik yazım (audit #11): aynı dizinde temp dosyaya yaz + fsync + rename → yazım ortasında
/// çökme/disk-dolması kullanıcı notunu yarım/bozuk bırakmaz (settings.rs ile aynı desen).
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    // Sayaç (codex): aynı nota aynı süreçte eşzamanlı yazımlar AYNI temp dosyayı paylaşmasın.
    static TMP_SEQ: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid note path: {}", path.display()))?;
    let tmp = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("note.md"),
        std::process::id(),
        TMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let write = || -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    };
    if let Err(err) = write() {
        let _ = fs::remove_file(&tmp);
        return Err(format!("failed to write {}: {err}", path.display()));
    }
    fs::rename(&tmp, path).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        format!("failed to replace {}: {err}", path.display())
    })
}

/// AI çıktısını (plan/cevap/inceleme) AKTİF workspace'in "AURA/" klasörüne not olarak
/// kaydeder. Plan→eylem köprüsü: çıktı artık ölü-uçta kalmaz. Kaydedilen yolun döner.
#[tauri::command]
pub fn save_note(kind: String, content: String) -> Result<String, String> {
    let settings = settings::load();
    let root = settings
        .active_root()
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
    atomic_write(&path, &content)?;
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

/// Workspace semantiği: okuma/yazma yalnız AKTİF kökün altında serbesttir. Son-kullanılan
/// ama aktif olmayan repo'lar da reddedilir (UI onları zaten göstermez; burası derin savunma).
fn is_under_vault_root(path: &Path, settings: &Settings) -> Result<bool, String> {
    let Some(active) = settings.active_root() else {
        return Ok(false);
    };

    let Ok(root) = PathBuf::from(active).canonicalize() else {
        return Ok(false);
    };
    Ok(path.starts_with(root))
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
