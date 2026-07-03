use crate::graph::{self, GraphData};
use crate::indexer::{IndexStats, Indexer, SearchHit};
use crate::ReadDb;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

// PERF (audit C7/C21/C22): bu komutlar eskiden SYNC'ti → Tauri ana thread'inde koşup
// Indexer kilidini beklerken TÜM event-loop donuyordu (indeksleme sırasında arama =
// tam UI donması). async + spawn_blocking: kilit beklemesi artık havuz thread'inde.
#[tauri::command]
pub async fn index_vault(app: tauri::AppHandle, path: String) -> Result<IndexStats, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Mutex<Indexer>>();
        let mut indexer = state.lock().map_err(|err| err.to_string())?;
        indexer.index_vault(&PathBuf::from(path))
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn get_graph(read: State<'_, ReadDb>) -> Result<GraphData, String> {
    // Ayrı read connection → indeksleme sürerken graph DONMAZ (codex #2 güvenli dilim).
    let settings = crate::settings::load();
    let conn = read.0.lock().map_err(|err| err.to_string())?;
    // Deferred read-tx: iki SELECT (files+links) tutarlı tek snapshot'tan okunur.
    // audit C10: commit başarısız olursa (nadir) rollback ile bağlantıyı açık
    // transaction'da BIRAKMA — sonraki list_notes bayat snapshot'a kilitlenmesin.
    let _ = conn.begin();
    // Workspace semantiği: graph yalnız AKTİF kökün dosya/bağlantılarından kurulur.
    let result = graph::build_from_db_under(&conn, settings.active_root());
    if conn.commit().is_err() {
        let _ = conn.rollback();
    }
    // audit C10: DB hatasını boş grafa çevirip yutma — UI gerçek hatayı görsün.
    result.map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn search_fts(
    app: tauri::AppHandle,
    query: String,
    k: u32,
) -> Result<Vec<SearchHit>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let settings = crate::settings::load();
        let state = app.state::<Mutex<Indexer>>();
        let indexer = state.lock().map_err(|err| err.to_string())?;
        indexer.search_fts_in(&query, (k as usize).clamp(1, 50), settings.active_root())
    })
    .await
    .map_err(|err| err.to_string())?
}
