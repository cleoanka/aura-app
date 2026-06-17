use crate::env_resolver;
use command_group::{CommandGroup, GroupChild};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{ChildStdout, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::ipc::Channel;

const JOB_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Serialize, Clone)]
#[serde(tag = "kind")]
pub enum AiEvent {
    Start { lane: String },
    Chunk { text: String },
    Cached { text: String },
    Done { run_dir: Option<String> },
    Error { reason: String, taxonomy: String },
}

#[derive(Clone)]
pub struct JobHandle {
    child: Arc<Mutex<GroupChild>>,
    cancelled: Arc<AtomicBool>,
}

pub type JobRegistry = Arc<Mutex<HashMap<String, JobHandle>>>;

pub fn new_job_registry() -> JobRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn run_aura(
    job_id: String,
    lane: &str,
    prompt: &str,
    context: &str,
    on_event: Channel<AiEvent>,
    jobs: JobRegistry,
) -> Result<String, String> {
    let temp_paths = write_job_files(&job_id, prompt, context)?;
    let result = run_aura_with_files(
        job_id.clone(),
        lane,
        &temp_paths.prompt,
        &temp_paths.context,
        on_event.clone(),
        jobs.clone(),
    )
    .await;

    let _ = fs::remove_file(&temp_paths.prompt);
    let _ = fs::remove_file(&temp_paths.context);
    result
}

pub fn cancel(jobs: JobRegistry, job_id: &str) {
    let handle = jobs.lock().ok().and_then(|jobs| jobs.get(job_id).cloned());

    if let Some(handle) = handle {
        handle.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut child) = handle.child.lock() {
            let _ = child.kill();
        }
    }
}

async fn run_aura_with_files(
    job_id: String,
    lane: &str,
    prompt_path: &PathBuf,
    context_path: &PathBuf,
    on_event: Channel<AiEvent>,
    jobs: JobRegistry,
) -> Result<String, String> {
    let mut command = Command::new("aura");
    let prompt_arg = prompt_path.to_string_lossy().into_owned();
    let context_arg = context_path.to_string_lossy().into_owned();
    command
        .env_clear()
        .envs(env_resolver::login_env())
        .args(["--lane", lane])
        .args(["--prompt-file", &prompt_arg])
        .args(["--context", &context_arg])
        .arg("--json-events")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    let mut child = command
        .group_spawn()
        .map_err(|err| format!("failed to start aura: {err}"))?;
    let stdout = child
        .inner()
        .stdout
        .take()
        .ok_or_else(|| "failed to capture aura stdout".to_string())?;
    let handle = JobHandle {
        child: Arc::new(Mutex::new(child)),
        cancelled: Arc::new(AtomicBool::new(false)),
    };

    jobs.lock()
        .map_err(|err| err.to_string())?
        .insert(job_id.clone(), handle.clone());

    let read_task = tokio::task::spawn_blocking({
        let on_event = on_event.clone();
        let cancelled = handle.cancelled.clone();
        let lane = lane.to_string();
        move || read_jsonl(stdout, lane, on_event, cancelled)
    });

    let read_result = match tokio::time::timeout(JOB_TIMEOUT, read_task).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(format!("aura reader task failed: {err}")),
        Err(_) => {
            handle.cancelled.store(true, Ordering::SeqCst);
            if let Ok(mut child) = handle.child.lock() {
                let _ = child.kill();
            }
            let event = AiEvent::Error {
                reason: "aura timed out".to_string(),
                taxonomy: "network".to_string(),
            };
            let _ = on_event.send(event);
            Err("aura timed out".to_string())
        }
    };

    if let Ok(mut jobs) = jobs.lock() {
        jobs.remove(&job_id);
    }

    {
        let child = handle.child.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut child) = child.lock() {
                let _ = child.wait();
            }
        })
        .await;
    }

    read_result
}

fn read_jsonl(
    stdout: ChildStdout,
    default_lane: String,
    on_event: Channel<AiEvent>,
    cancelled: Arc<AtomicBool>,
) -> Result<String, String> {
    let lines = BufReader::new(stdout).lines();
    let mut text = String::new();

    for line in lines {
        let line = line.map_err(|err| format!("failed to read aura output: {err}"))?;
        let event: AuraJsonEvent = serde_json::from_str(&line)
            .map_err(|err| format!("failed to parse aura JSONL event: {err}"))?;

        match event {
            AuraJsonEvent::Start { lane, .. } => {
                send_event(
                    &on_event,
                    AiEvent::Start {
                        lane: lane.unwrap_or_else(|| default_lane.clone()),
                    },
                )?;
            }
            AuraJsonEvent::Chunk { text: chunk } => {
                text.push_str(&chunk);
                send_event(&on_event, AiEvent::Chunk { text: chunk })?;
            }
            AuraJsonEvent::Done { ok, run_dir } => {
                if ok.unwrap_or(true) {
                    send_event(&on_event, AiEvent::Done { run_dir })?;
                    return Ok(text);
                }
                let reason = "aura completed unsuccessfully".to_string();
                send_event(
                    &on_event,
                    AiEvent::Error {
                        reason: reason.clone(),
                        taxonomy: "unknown".to_string(),
                    },
                )?;
                return Err(reason);
            }
            AuraJsonEvent::Error { reason, taxonomy } => {
                let reason = reason.unwrap_or_else(|| "aura returned an error".to_string());
                let taxonomy = taxonomy.unwrap_or_else(|| "unknown".to_string());
                send_event(
                    &on_event,
                    AiEvent::Error {
                        reason: reason.clone(),
                        taxonomy,
                    },
                )?;
                return Err(reason);
            }
            AuraJsonEvent::Other => {}
        }
    }

    if cancelled.load(Ordering::SeqCst) {
        let reason = "aura job cancelled".to_string();
        send_event(
            &on_event,
            AiEvent::Error {
                reason: reason.clone(),
                taxonomy: "cancelled".to_string(),
            },
        )?;
        Err(reason)
    } else {
        Err("aura exited before sending a done event".to_string())
    }
}

fn send_event(on_event: &Channel<AiEvent>, event: AiEvent) -> Result<(), String> {
    on_event
        .send(event)
        .map_err(|err| format!("failed to send AI event: {err}"))
}

struct JobTempPaths {
    prompt: PathBuf,
    context: PathBuf,
}

fn write_job_files(job_id: &str, prompt: &str, context: &str) -> Result<JobTempPaths, String> {
    let dir = temp_dir()?;
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create aura temp dir: {err}"))?;
    set_dir_permissions(&dir)?;

    let safe_job_id = safe_job_id(job_id);
    let prompt_path = dir.join(format!("{safe_job_id}.prompt"));
    let context_path = dir.join(format!("{safe_job_id}.context"));

    write_private_file(&prompt_path, prompt)?;
    write_private_file(&context_path, context)?;

    Ok(JobTempPaths {
        prompt: prompt_path,
        context: context_path,
    })
}

fn temp_dir() -> Result<PathBuf, String> {
    let mut dir = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("aura-desktop");
    dir.push("tmp");
    Ok(dir)
}

fn write_private_file(path: &PathBuf, content: &str) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn set_dir_permissions(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|err| format!("failed to set aura temp dir permissions: {err}"))
}

#[cfg(not(unix))]
fn set_dir_permissions(_path: &PathBuf) -> Result<(), String> {
    Ok(())
}

fn safe_job_id(job_id: &str) -> String {
    job_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>()
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AuraJsonEvent {
    #[serde(rename = "start")]
    Start {
        #[serde(default)]
        lane: Option<String>,
    },
    #[serde(rename = "chunk")]
    Chunk { text: String },
    #[serde(rename = "done")]
    Done {
        #[serde(default)]
        ok: Option<bool>,
        #[serde(default)]
        run_dir: Option<String>,
    },
    #[serde(rename = "error")]
    Error {
        #[serde(default)]
        reason: Option<String>,
        #[serde(default)]
        taxonomy: Option<String>,
    },
    #[serde(other)]
    Other,
}
