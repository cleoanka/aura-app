pub mod agent;
mod agent_manager;
mod commands;
mod env_resolver;
mod error;

use commands::{agent_detect, agent_install};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, agent_detect, agent_install])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
