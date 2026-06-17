use crate::exec::{self, AiEvent, JobRegistry};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{ipc::Channel, AppHandle, State};

static MODE_JOB_COUNTER: AtomicU64 = AtomicU64::new(1);

#[tauri::command]
pub async fn run_mode(
    _app: AppHandle,
    jobs: State<'_, JobRegistry>,
    mode: String,
    prompt: String,
    project_dir: Option<String>,
    on_event: Channel<AiEvent>,
) -> Result<String, String> {
    exec::run_aura_mode(
        new_mode_job_id(),
        &mode,
        &prompt,
        project_dir.as_deref(),
        on_event,
        jobs.inner().clone(),
    )
    .await
}

fn new_mode_job_id() -> String {
    let counter = MODE_JOB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("mode-{}-{nanos:x}-{counter:x}", std::process::id())
}
