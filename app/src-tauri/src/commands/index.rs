use crate::graph::{self, GraphData};
use crate::indexer::{IndexStats, Indexer, SearchHit};
use crate::ReadDb;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn index_vault(indexer: State<'_, Mutex<Indexer>>, path: String) -> Result<IndexStats, String> {
    let mut indexer = indexer.lock().map_err(|err| err.to_string())?;
    indexer.index_vault(&PathBuf::from(path))
}

#[tauri::command]
pub fn get_graph(read: State<'_, ReadDb>) -> Result<GraphData, String> {
    // Ayrı read connection → indeksleme sürerken graph DONMAZ (codex #2 güvenli dilim).
    let conn = read.0.lock().map_err(|err| err.to_string())?;
    // Deferred read-tx: iki SELECT (files+links) tutarlı tek snapshot'tan okunur.
    let _ = conn.begin();
    let result = graph::build_from_db(&conn);
    let _ = conn.commit();
    Ok(result.unwrap_or_default())
}

#[tauri::command]
pub fn search_fts(
    indexer: State<'_, Mutex<Indexer>>,
    query: String,
    k: u32,
) -> Result<Vec<SearchHit>, String> {
    let indexer = indexer.lock().map_err(|err| err.to_string())?;
    indexer.search_fts(&query, (k as usize).clamp(1, 50))
}
