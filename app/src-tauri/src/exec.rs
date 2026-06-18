use crate::env_resolver;
use command_group::{CommandGroup, GroupChild};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdout, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::ipc::Channel;

const JOB_TIMEOUT: Duration = Duration::from_secs(600);
const MODE_PROMPT_PLACEHOLDER: &str = "<prompt-file>";

#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AiEvent {
    Start { lane: String },
    /// İş kimliği baştan UI'a gönderilir → Stop butonu akış sırasında çalışır.
    Job { job_id: String },
    Chunk { text: String },
    Cached { text: String },
    Status {
        text: String,
        stage: Option<String>,
        agent: Option<String>,
    },
    Done { run_dir: Option<String> },
    Error { reason: String, taxonomy: String },
}

#[derive(Clone)]
pub struct JobHandle {
    children: Arc<Mutex<Vec<Arc<Mutex<GroupChild>>>>>,
    cancelled: Arc<AtomicBool>,
}

pub type JobRegistry = Arc<Mutex<HashMap<String, JobHandle>>>;

impl Default for JobHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl JobHandle {
    pub fn new() -> Self {
        Self {
            children: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn add_child(&self, child: GroupChild) -> Arc<Mutex<GroupChild>> {
        let child = Arc::new(Mutex::new(child));
        if let Ok(mut children) = self.children.lock() {
            children.push(child.clone());
        }
        if self.is_cancelled() {
            if let Ok(mut child) = child.lock() {
                let _ = child.kill();
            }
        }
        child
    }

    pub fn cancelled(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(children) = self.children.lock() {
            for child in children.iter() {
                if let Ok(mut child) = child.lock() {
                    let _ = child.kill();
                }
            }
        }
    }
}

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

pub async fn run_aura_mode(
    job_id: String,
    mode: &str,
    prompt: &str,
    project_dir: Option<&str>,
    on_event: Channel<AiEvent>,
    jobs: JobRegistry,
) -> Result<String, String> {
    let has_prompt = !prompt.trim().is_empty();
    let argv = build_mode_argv(mode, has_prompt)?;
    let prompt_path = if has_prompt {
        let path = mode_prompt_path(&job_id)?;
        write_private_file(&path, prompt)?;
        Some(path)
    } else {
        None
    };

    let result = run_aura_mode_with_argv(
        job_id,
        mode,
        &argv,
        prompt_path.as_ref(),
        project_dir,
        on_event,
        jobs,
    )
    .await;

    if let Some(path) = prompt_path {
        let _ = fs::remove_file(path);
    }

    result
}

pub fn build_mode_argv(mode: &str, has_prompt: bool) -> Result<Vec<String>, String> {
    if !matches!(mode, "plan" | "review" | "fix" | "ship") {
        return Err(format!("unsupported aura mode: {mode}"));
    }

    let mut argv = vec!["aura".to_string(), mode.to_string()];
    if has_prompt {
        argv.push("--prompt-file".to_string());
        argv.push(MODE_PROMPT_PLACEHOLDER.to_string());
    }
    // Tüm modlar artık --json-events (verbose status + canlı token akışı).
    argv.push("--json-events".to_string());
    Ok(argv)
}

pub fn cancel(jobs: JobRegistry, job_id: &str) {
    let handle = jobs.lock().ok().and_then(|jobs| jobs.get(job_id).cloned());

    if let Some(handle) = handle {
        handle.cancel();
    }
}

async fn run_aura_mode_with_argv(
    job_id: String,
    mode: &str,
    argv: &[String],
    prompt_path: Option<&PathBuf>,
    project_dir: Option<&str>,
    on_event: Channel<AiEvent>,
    jobs: JobRegistry,
) -> Result<String, String> {
    let program = argv
        .first()
        .ok_or_else(|| "aura mode argv is empty".to_string())?;
    let prompt_arg = prompt_path.map(|path| path.to_string_lossy().into_owned());
    let args = argv
        .iter()
        .skip(1)
        .map(|arg| {
            if arg == MODE_PROMPT_PLACEHOLDER {
                prompt_arg.clone().unwrap_or_default()
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>();

    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(env_resolver::login_env())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    if let Some(project_dir) = project_dir.filter(|dir| !dir.trim().is_empty()) {
        command.current_dir(project_dir);
    }

    let mut child = command
        .group_spawn()
        .map_err(|err| format!("failed to start aura: {err}"))?;
    let stdout = child
        .inner()
        .stdout
        .take()
        .ok_or_else(|| "failed to capture aura stdout".to_string())?;
    // Tüm modlar artık json-events → her zaman read_jsonl (verbose + canlı).
    let raw_stdout_mode = false;
    let handle = JobHandle::new();
    let child = handle.add_child(child);

    jobs.lock()
        .map_err(|err| err.to_string())?
        .insert(job_id.clone(), handle.clone());
    send_event(&on_event, AiEvent::Job { job_id: job_id.clone() })?;

    let read_task = tokio::task::spawn_blocking({
        let on_event = on_event.clone();
        let cancelled = handle.cancelled();
        let lane = mode.to_string();
        move || read_jsonl(stdout, lane, on_event, cancelled)
    });

    let read_result = match tokio::time::timeout(JOB_TIMEOUT, read_task).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(format!("aura reader task failed: {err}")),
        Err(_) => {
            handle.cancel();
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

    let wait_result = {
        tokio::task::spawn_blocking(move || {
            if let Ok(mut child) = child.lock() {
                child
                    .wait()
                    .map_err(|err| format!("failed to wait for aura: {err}"))
            } else {
                Err("failed to lock aura process while waiting".to_string())
            }
        })
        .await
        .map_err(|err| format!("aura wait task failed: {err}"))?
    };

    if raw_stdout_mode {
        match (read_result, wait_result) {
            (Ok(text), Ok(status)) if status.success() => {
                send_event(&on_event, AiEvent::Done { run_dir: None })?;
                Ok(text)
            }
            (Ok(_), Ok(status)) => {
                let reason = format!("aura exited with status {status}");
                send_event(
                    &on_event,
                    AiEvent::Error {
                        reason: reason.clone(),
                        taxonomy: "process".to_string(),
                    },
                )?;
                Err(reason)
            }
            (Ok(_), Err(reason)) => {
                send_event(
                    &on_event,
                    AiEvent::Error {
                        reason: reason.clone(),
                        taxonomy: "process".to_string(),
                    },
                )?;
                Err(reason)
            }
            (Err(reason), _) => Err(reason),
        }
    } else {
        read_result
    }
}

async fn run_aura_with_files(
    job_id: String,
    lane: &str,
    prompt_path: &Path,
    context_path: &Path,
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
        .arg("--answer") // ask/chat: planla DEĞİL, soruyu cevapla
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
    let handle = JobHandle::new();
    let child = handle.add_child(child);

    jobs.lock()
        .map_err(|err| err.to_string())?
        .insert(job_id.clone(), handle.clone());
    send_event(&on_event, AiEvent::Job { job_id: job_id.clone() })?;

    let read_task = tokio::task::spawn_blocking({
        let on_event = on_event.clone();
        let cancelled = handle.cancelled();
        let lane = lane.to_string();
        move || read_jsonl(stdout, lane, on_event, cancelled)
    });

    let read_result = match tokio::time::timeout(JOB_TIMEOUT, read_task).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(format!("aura reader task failed: {err}")),
        Err(_) => {
            handle.cancel();
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
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut child) = child.lock() {
                let _ = child.wait();
            }
        })
        .await;
    }

    read_result
}

// Tüm modlar artık json-events kullanıyor; ham-satır modu ileride lazım olursa diye duruyor.
#[allow(dead_code)]
fn read_stdout_lines(
    stdout: ChildStdout,
    lane: String,
    on_event: Channel<AiEvent>,
    cancelled: Arc<AtomicBool>,
) -> Result<String, String> {
    send_event(&on_event, AiEvent::Start { lane })?;

    let lines = BufReader::new(stdout).lines();
    let mut text = String::new();

    for line in lines {
        if cancelled.load(Ordering::SeqCst) {
            break;
        }
        let mut line = match line {
            Ok(line) => line,
            Err(err) => {
                if cancelled.load(Ordering::SeqCst) {
                    break;
                }
                return Err(format!("failed to read aura output: {err}"));
            }
        };
        line.push('\n');
        text.push_str(&line);
        send_event(&on_event, AiEvent::Chunk { text: line })?;
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
        Ok(text)
    }
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
        // İptal/timeout sonrası (cancel() cancelled=true yapar) artık event YAYMA.
        if cancelled.load(Ordering::SeqCst) {
            break;
        }
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                // kill stdout'u kestiyse: IO hatası DEĞİL, iptal say.
                if cancelled.load(Ordering::SeqCst) {
                    break;
                }
                return Err(format!("failed to read aura output: {err}"));
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        // DAYANIKLILIK: JSON olmayan satır akışı ÖLDÜRMESİN — atla, devam et.
        // (eski hali parse hatasında tüm streaming'i bitiriyordu.)
        let event: AuraJsonEvent = match serde_json::from_str(&line) {
            Ok(event) => event,
            Err(_) => continue,
        };

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
            AuraJsonEvent::Status { text, stage, agent } => {
                send_event(&on_event, AiEvent::Status { text, stage, agent })?;
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

fn mode_prompt_path(job_id: &str) -> Result<PathBuf, String> {
    let dir = temp_dir()?;
    fs::create_dir_all(&dir).map_err(|err| format!("failed to create aura temp dir: {err}"))?;
    set_dir_permissions(&dir)?;
    Ok(dir.join(format!("{}.mode.prompt", safe_job_id(job_id))))
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
    #[serde(rename = "status")]
    Status {
        text: String,
        #[serde(default)]
        stage: Option<String>,
        #[serde(default)]
        agent: Option<String>,
    },
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

#[cfg(test)]
mod tests {
    use super::*;

    // KRİTİK SÖZLEŞME: AiEvent.kind frontend ile birebir lowercase eşleşmeli.
    // (PascalCase olursa UI'daki switch hiçbir case'i tutmaz → "Başlatılıyor" donar.)
    fn kind(event: &AiEvent) -> String {
        serde_json::to_value(event).unwrap()["kind"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn ai_event_kind_is_lowercase_for_frontend() {
        assert_eq!(kind(&AiEvent::Start { lane: "fast".into() }), "start");
        assert_eq!(kind(&AiEvent::Job { job_id: "j".into() }), "job");
        assert_eq!(kind(&AiEvent::Chunk { text: "t".into() }), "chunk");
        assert_eq!(kind(&AiEvent::Cached { text: "t".into() }), "cached");
        assert_eq!(
            kind(&AiEvent::Status {
                text: "t".into(),
                stage: None,
                agent: None
            }),
            "status"
        );
        assert_eq!(kind(&AiEvent::Done { run_dir: None }), "done");
        assert_eq!(
            kind(&AiEvent::Error {
                reason: "r".into(),
                taxonomy: "t".into()
            }),
            "error"
        );
    }

    #[test]
    fn ai_event_job_carries_job_id_field() {
        let value = serde_json::to_value(AiEvent::Job { job_id: "abc".into() }).unwrap();
        assert_eq!(value["job_id"], "abc");
    }
}
