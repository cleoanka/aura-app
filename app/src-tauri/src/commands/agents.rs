use crate::agent::DoctorReport;
use crate::agent_manager;

// PERF (audit C6/C17): sync Tauri komutu ANA thread'de koşar. `aura doctor --probe`
// (saniyeler) ve `npm i -g` (dakikalar) UI'ı komple donduruyordu → async + spawn_blocking.
#[tauri::command]
pub async fn agent_detect(probe: bool) -> Result<DoctorReport, String> {
    tauri::async_runtime::spawn_blocking(move || agent_manager::detect(probe).map_err(Into::into))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn agent_install(id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || agent_manager::install(&id).map_err(Into::into))
        .await
        .map_err(|err| err.to_string())?
}

/// Tek bir ajanı izole test et (AI&Models "Test" butonu): cevap + gecikme.
#[tauri::command]
pub async fn agent_test(agent: String) -> Result<crate::consensus::AgentTestResult, String> {
    Ok(crate::consensus::test_agent(agent).await)
}
