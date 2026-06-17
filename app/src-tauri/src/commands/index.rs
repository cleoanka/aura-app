use crate::graph::GraphData;
use crate::indexer::{IndexStats, Indexer, SearchHit};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

#[tauri::command]
pub fn index_vault(indexer: State<'_, Mutex<Indexer>>, path: String) -> Result<IndexStats, String> {
    let mut indexer = indexer.lock().map_err(|err| err.to_string())?;
    indexer.index_vault(&PathBuf::from(path))
}

#[tauri::command]
pub fn get_graph(indexer: State<'_, Mutex<Indexer>>) -> Result<GraphData, String> {
    let indexer = indexer.lock().map_err(|err| err.to_string())?;
    Ok(indexer.graph())
}

#[tauri::command]
pub fn search_fts(
    indexer: State<'_, Mutex<Indexer>>,
    query: String,
    k: u32,
) -> Result<Vec<SearchHit>, String> {
    let indexer = indexer.lock().map_err(|err| err.to_string())?;
    indexer.search_fts(&query, k as usize)
}
