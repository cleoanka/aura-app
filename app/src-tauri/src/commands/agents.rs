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
