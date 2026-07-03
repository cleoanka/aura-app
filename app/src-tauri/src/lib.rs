pub mod agent;
mod agent_manager;
pub mod apikey;
pub mod commands;
pub mod consensus;
pub mod db;
pub mod embed;
mod env_resolver;
pub mod error;
pub mod exec;
pub mod graph;
pub mod indexer;
pub mod lane0;
pub mod links;
pub mod markdown;
pub mod pty;
pub mod retrieval;
pub mod search;
pub mod settings;

use commands::{
    agent_detect, agent_install, agent_test, api_key_status, ask, ask_consensus, cancel_job, chat,
    clear_api_key, embedding_status, forget_workspace, get_graph, get_settings, get_workspace,
    index_vault, list_notes, ollama_pull, ollama_status, pick_vault_folder,
    prepare_embedding_model, pty_close, pty_open, pty_resize, pty_write, read_note, run_mode,
    save_note, search_fts, search_hybrid, set_active_workspace, set_api_key, set_settings,
    write_note,
};
use embed::default_embedder;
use indexer::Indexer;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // audit C8: DB açılamazsa (disk dolu, izin, bozuk dosya) panic yerine kullanıcıya
    // NEDENİ söyleyen bir dialog göster ve düzgün çık — "uygulama hiç açılmıyor" olmasın.
    let (indexer, read_db) = match create_indexer_state().and_then(|idx| {
        let read = create_read_db()?;
        Ok((idx, read))
    }) {
        Ok(pair) => pair,
        Err(err) => {
            fatal_startup_error(&format!(
                "AURA veritabanını açamadı:\n{err}\n\nDisk alanını/izinleri kontrol edin veya \
                 ~/Library/Application Support/aura-app/index.sqlite3 dosyasını taşıyıp yeniden deneyin."
            ));
            return;
        }
    };

    tauri::Builder::default()
        .manage(indexer)
        .manage(read_db)
        .manage(exec::new_job_registry())
        .setup(|app| {
            // Başlangıçta YALNIZ AKTİF workspace'i ARKA PLANDA yeniden indeksle
            // (güncel code-aware kodla; stale veriyi self-heal eder). Pencereyi bloklamaz.
            // PERF: eski davranış TÜM geçmiş kökleri tarıyordu → açılışta uzun süren
            // disk+CPU yükü ("uygulama kasıyor"). Repo mantığında tek aktif kök yeter;
            // pasif bir köke geçişte UI set_active_workspace + index_vault çağırır.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let settings = settings::load();
                let Some(active) = settings.active_root() else {
                    return;
                };
                let state = handle.state::<Mutex<Indexer>>();
                // 1) Hızlı indeksleme (embedding YOK) → dosyalar/graph/FTS hemen hazır.
                // audit S11: hata sessizce yutulmaz — UI'a index-error event'i + stderr log;
                // "index-updated" yalnız BAŞARILI indekste yayınlanır (UI yanlış "güncel" sanmasın).
                let outcome = match state.lock() {
                    Ok(mut idx) => idx.index_vault(&std::path::PathBuf::from(active)),
                    Err(err) => Err(format!("indexer lock poisoned: {err}")),
                };
                match outcome {
                    Ok(_) => {
                        let _ = handle.emit("index-updated", ());
                    }
                    Err(reason) => {
                        eprintln!("warning: startup reindex failed for {active}: {reason}");
                        let _ = handle.emit("index-error", reason);
                    }
                }

                // 2) Vektörleri SADECE semantic_search açıksa arka planda doldur.
                // Kapalıyken (varsayılan) embedding yok → CPU yükü yok, arama FTS5 ile.
                // Açıkken bile NAZİK throttle (her batch arası uyku) ile CPU pegleme yok.
                if settings::load().semantic_search {
                    loop {
                        let done = match state.lock() {
                            Ok(mut idx) => idx.embed_pending(16).unwrap_or(0),
                            Err(_) => 0,
                        };
                        if done == 0 {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(150));
                    }
                    let _ = handle.emit("index-updated", ());
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            agent_detect,
            agent_install,
            agent_test,
            embedding_status,
            prepare_embedding_model,
            ollama_status,
            ollama_pull,
            ask,
            ask_consensus,
            cancel_job,
            chat,
            run_mode,
            index_vault,
            get_graph,
            search_fts,
            search_hybrid,
            list_notes,
            read_note,
            write_note,
            save_note,
            pick_vault_folder,
            get_workspace,
            set_active_workspace,
            forget_workspace,
            get_settings,
            set_settings,
            pty_open,
            pty_write,
            pty_resize,
            pty_close,
            api_key_status,
            set_api_key,
            clear_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Açılış-anı ölümcül hatası: pencere yokken native dialog ile bildir (stderr'e de yaz).
fn fatal_startup_error(message: &str) {
    eprintln!("fatal: {message}");
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display alert \"AURA başlatılamadı\" message {:?} as critical",
            message
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status();
    }
}

/// PERF (codex #2 güvenli dilim): saf-okuma komutları (get_graph, list_notes) için
/// AYRI read connection. WAL eşzamanlı okumaya izin verir → arka planda indekslerken
/// (Indexer write-lock'u tutulurken) graph/dosya-listesi UI'ı DONMAZ. Ayrı kilit = deadlock yok.
pub struct ReadDb(pub Mutex<db::Connection>);

fn db_file_path() -> Result<std::path::PathBuf, String> {
    let mut db_dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(std::env::temp_dir);
    db_dir.push("aura-app");
    std::fs::create_dir_all(&db_dir).map_err(|err| err.to_string())?;
    Ok(db_dir.join("index.sqlite3"))
}

fn create_indexer_state() -> Result<Mutex<Indexer>, String> {
    let db_path = db_file_path()?;
    let conn = db::open(&db_path).map_err(|err| err.to_string())?;
    Ok(Mutex::new(Indexer::new(conn, default_embedder(), 1)))
}

fn create_read_db() -> Result<ReadDb, String> {
    let db_path = db_file_path()?;
    let conn = db::open(&db_path).map_err(|err| err.to_string())?;
    Ok(ReadDb(Mutex::new(conn)))
}
