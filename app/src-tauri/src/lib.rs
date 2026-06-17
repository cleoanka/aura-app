pub mod agent;
mod agent_manager;
pub mod commands;
pub mod consensus;
pub mod db;
pub mod embed;
mod env_resolver;
mod error;
pub mod exec;
pub mod graph;
pub mod indexer;
pub mod lane0;
pub mod links;
pub mod markdown;
pub mod pty;
pub mod search;
pub mod settings;

use commands::{
    agent_detect, agent_install, ask, ask_consensus, cancel_job, get_graph, get_settings,
    index_vault, list_notes, pick_vault_folder, pty_close, pty_open, pty_resize, pty_write,
    read_note, run_mode, search_fts, search_hybrid, set_settings, write_note,
};
use embed::default_embedder;
use indexer::Indexer;
use std::sync::Mutex;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let indexer = create_indexer_state().expect("failed to initialize indexer");

    tauri::Builder::default()
        .manage(indexer)
        .manage(exec::new_job_registry())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            agent_detect,
            agent_install,
            ask,
            ask_consensus,
            cancel_job,
            run_mode,
            index_vault,
            get_graph,
            search_fts,
            search_hybrid,
            list_notes,
            read_note,
            write_note,
            pick_vault_folder,
            get_settings,
            set_settings,
            pty_open,
            pty_write,
            pty_resize,
            pty_close
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn create_indexer_state() -> Result<Mutex<Indexer>, String> {
    let mut db_dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(std::env::temp_dir);
    db_dir.push("aura-app");
    std::fs::create_dir_all(&db_dir).map_err(|err| err.to_string())?;
    let db_path = db_dir.join("index.sqlite3");
    let conn = db::open(&db_path).map_err(|err| err.to_string())?;
    Ok(Mutex::new(Indexer::new(conn, default_embedder(), 1)))
}
