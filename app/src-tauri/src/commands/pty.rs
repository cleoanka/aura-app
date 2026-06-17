use tauri::ipc::Channel;

#[tauri::command]
pub fn pty_open(agent: String, on_output: Channel<String>) -> Result<String, String> {
    crate::pty::open(&agent, on_output)
}

#[tauri::command]
pub fn pty_write(session_id: String, data: String) -> Result<(), String> {
    crate::pty::write(&session_id, &data)
}

#[tauri::command]
pub fn pty_resize(session_id: String, rows: u16, cols: u16) -> Result<(), String> {
    crate::pty::resize(&session_id, rows, cols)
}

#[tauri::command]
pub fn pty_close(session_id: String) -> Result<(), String> {
    crate::pty::close(&session_id)
}
