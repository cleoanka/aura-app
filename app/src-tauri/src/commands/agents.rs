use crate::agent::DoctorReport;
use crate::agent_manager;

#[tauri::command]
pub fn agent_detect(probe: bool) -> Result<DoctorReport, String> {
    agent_manager::detect(probe).map_err(Into::into)
}

#[tauri::command]
pub fn agent_install(id: String) -> Result<String, String> {
    agent_manager::install(&id).map_err(Into::into)
}

/// Tek bir ajanı izole test et (AI&Models "Test" butonu): cevap + gecikme.
#[tauri::command]
pub async fn agent_test(agent: String) -> Result<crate::consensus::AgentTestResult, String> {
    Ok(crate::consensus::test_agent(agent).await)
}
