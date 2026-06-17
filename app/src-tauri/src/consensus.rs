use crate::env_resolver;
use crate::exec::{AiEvent, JobHandle, JobRegistry};
use command_group::{CommandGroup, GroupChild};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdout, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::ipc::Channel;

const TOTAL_TIMEOUT: Duration = Duration::from_secs(600);
const AGENT_TIMEOUT: Duration = Duration::from_secs(420);
const SYNTHESIS_TIMEOUT: Duration = Duration::from_secs(240);

pub fn synth_prompt(query: &str, answers: &[(String, String)]) -> String {
    let mut prompt = String::new();
    prompt.push_str("ANA BEYIN consensus synthesis task.\n");
    prompt.push_str("You will receive one user question and multiple agent answers.\n");
    prompt.push_str(
        "Note agreements, flag material conflicts, then produce ONE best synthesized answer. Keep it concise.\n\n",
    );
    prompt.push_str("USER QUESTION:\n<<<QUERY\n");
    prompt.push_str(query);
    prompt.push_str("\nQUERY\n\n");

    for (agent, answer) in answers {
        prompt.push_str("AGENT ANSWER: ");
        prompt.push_str(agent);
        prompt.push_str("\n<<<ANSWER\n");
        prompt.push_str(answer);
        prompt.push_str("\nANSWER\n\n");
    }

    prompt.push_str("SYNTHESIZED CONSENSUS ANSWER:\n");
    prompt
}

pub async fn run_consensus(
    job_id: String,
    query: &str,
    context: &str,
    on_event: Channel<AiEvent>,
    jobs: JobRegistry,
) -> Result<String, String> {
    send_event(
        &on_event,
        AiEvent::Start {
            lane: "consensus".to_string(),
        },
    )?;

    let temp_paths = ConsensusTempPaths::new(&job_id)?;
    let shared_prompt = format!("{context}\n\nSORU:\n{query}");
    write_private_file(&temp_paths.shared_prompt, &shared_prompt)?;

    let handle = JobHandle::new();
    jobs.lock()
        .map_err(|err| err.to_string())?
        .insert(job_id.clone(), handle.clone());

    let result = tokio::time::timeout(
        TOTAL_TIMEOUT,
        run_consensus_inner(
            query.to_string(),
            temp_paths.shared_prompt.clone(),
            temp_paths.synthesis_prompt.clone(),
            on_event.clone(),
            handle.clone(),
        ),
    )
    .await;

    let result = match result {
        Ok(result) => result,
        Err(_) => {
            handle.cancel();
            let reason = "consensus timed out".to_string();
            let _ = on_event.send(AiEvent::Error {
                reason: reason.clone(),
                taxonomy: "network".to_string(),
            });
            Err(reason)
        }
    };

    if let Ok(mut jobs) = jobs.lock() {
        jobs.remove(&job_id);
    }
    temp_paths.cleanup();

    result
}

async fn run_consensus_inner(
    query: String,
    shared_prompt_path: PathBuf,
    synthesis_prompt_path: PathBuf,
    on_event: Channel<AiEvent>,
    handle: JobHandle,
) -> Result<String, String> {
    let mut tasks = Vec::new();
    for agent in consensus_agents() {
        tasks.push(tokio::spawn(run_agent(
            agent,
            shared_prompt_path.clone(),
            on_event.clone(),
            handle.clone(),
        )));
    }

    let mut answers = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Some((agent, answer))) if !answer.trim().is_empty() => answers.push((agent, answer)),
            Ok(_) => {}
            Err(_) => {}
        }
    }

    if handle.is_cancelled() {
        let reason = "consensus job cancelled".to_string();
        send_event(
            &on_event,
            AiEvent::Error {
                reason: reason.clone(),
                taxonomy: "cancelled".to_string(),
            },
        )?;
        return Err(reason);
    }

    if answers.is_empty() {
        let reason = "all consensus agents failed".to_string();
        send_event(
            &on_event,
            AiEvent::Error {
                reason: reason.clone(),
                taxonomy: "model".to_string(),
            },
        )?;
        return Err(reason);
    }

    let synthesis_prompt = synth_prompt(&query, &answers);
    write_private_file(&synthesis_prompt_path, &synthesis_prompt)?;
    run_synthesis(&synthesis_prompt_path, on_event, handle).await
}

async fn run_agent(
    agent: AgentSpec,
    prompt_path: PathBuf,
    on_event: Channel<AiEvent>,
    handle: JobHandle,
) -> Option<(String, String)> {
    if handle.is_cancelled() {
        return None;
    }

    let child = match spawn_agent(&agent, &prompt_path, &handle) {
        Ok(child) => child,
        Err(_) => return None,
    };

    let stdout = {
        let mut child = child.lock().ok()?;
        child.inner().stdout.take()?
    };

    let read_task = tokio::task::spawn_blocking(move || read_stdout_to_string(stdout));
    let output = match tokio::time::timeout(AGENT_TIMEOUT, read_task).await {
        Ok(Ok(Ok(output))) => output,
        Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => {
            kill_child(&child);
            wait_child(child).await;
            return None;
        }
    };

    let status = wait_child(child).await;
    if !matches!(status, Some(status) if status.success()) || handle.is_cancelled() {
        return None;
    }

    let _ = on_event.send(AiEvent::Chunk {
        text: format!("\n— {} yanıtladı —\n", agent.name),
    });
    Some((agent.name.to_string(), output))
}

async fn run_synthesis(
    prompt_path: &Path,
    on_event: Channel<AiEvent>,
    handle: JobHandle,
) -> Result<String, String> {
    if handle.is_cancelled() {
        return Err("consensus job cancelled".to_string());
    }

    let agent = AgentSpec {
        name: "claude",
        program: "claude",
        args: &["-p"],
    };
    let child = spawn_agent(&agent, prompt_path, &handle)?;
    let stdout = {
        let mut child = child
            .lock()
            .map_err(|_| "failed to lock claude synthesis process".to_string())?;
        child
            .inner()
            .stdout
            .take()
            .ok_or_else(|| "failed to capture claude synthesis stdout".to_string())?
    };

    let read_task = tokio::task::spawn_blocking({
        let on_event = on_event.clone();
        let cancelled = handle.cancelled();
        move || stream_stdout(stdout, on_event, cancelled)
    });

    let mut timed_out = false;
    let read_result = match tokio::time::timeout(SYNTHESIS_TIMEOUT, read_task).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(format!("claude synthesis reader task failed: {err}")),
        Err(_) => {
            timed_out = true;
            handle.cancel();
            Err("claude synthesis timed out".to_string())
        }
    };

    let status = wait_child(child).await;

    match (read_result, status) {
        (Ok(text), Some(status)) if status.success() => {
            send_event(&on_event, AiEvent::Done { run_dir: None })?;
            Ok(text)
        }
        (Ok(_), Some(status)) => {
            let reason = format!("claude synthesis exited with status {status}");
            send_event(
                &on_event,
                AiEvent::Error {
                    reason: reason.clone(),
                    taxonomy: "process".to_string(),
                },
            )?;
            Err(reason)
        }
        (Ok(_), None) => {
            let reason = "failed to wait for claude synthesis".to_string();
            send_event(
                &on_event,
                AiEvent::Error {
                    reason: reason.clone(),
                    taxonomy: "process".to_string(),
                },
            )?;
            Err(reason)
        }
        (Err(reason), _) => {
            let taxonomy = if timed_out {
                "network"
            } else if handle.is_cancelled() {
                "cancelled"
            } else {
                "model"
            };
            send_event(
                &on_event,
                AiEvent::Error {
                    reason: reason.clone(),
                    taxonomy: taxonomy.to_string(),
                },
            )?;
            Err(reason)
        }
    }
}

fn spawn_agent(
    agent: &AgentSpec,
    prompt_path: &Path,
    handle: &JobHandle,
) -> Result<Arc<Mutex<GroupChild>>, String> {
    let stdin = File::open(prompt_path)
        .map(Stdio::from)
        .map_err(|err| format!("failed to open prompt file for {}: {err}", agent.name))?;
    let mut command = env_resolver::login_command(agent.program);
    command
        .args(agent.args)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let child = command
        .group_spawn()
        .map_err(|err| format!("failed to start {}: {err}", agent.name))?;
    Ok(handle.add_child(child))
}

fn read_stdout_to_string(stdout: ChildStdout) -> Result<String, String> {
    let mut reader = BufReader::new(stdout);
    let mut text = String::new();
    reader
        .read_to_string(&mut text)
        .map_err(|err| format!("failed to read agent output: {err}"))?;
    Ok(text)
}

fn stream_stdout(
    stdout: ChildStdout,
    on_event: Channel<AiEvent>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
) -> Result<String, String> {
    let mut text = String::new();
    for line in BufReader::new(stdout).lines() {
        let mut line =
            line.map_err(|err| format!("failed to read claude synthesis output: {err}"))?;
        line.push('\n');
        text.push_str(&line);
        send_event(&on_event, AiEvent::Chunk { text: line })?;
    }

    if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
        Err("consensus job cancelled".to_string())
    } else {
        Ok(text)
    }
}

async fn wait_child(child: Arc<Mutex<GroupChild>>) -> Option<std::process::ExitStatus> {
    tokio::task::spawn_blocking(move || {
        let mut child = child.lock().ok()?;
        child.wait().ok()
    })
    .await
    .ok()
    .flatten()
}

fn kill_child(child: &Arc<Mutex<GroupChild>>) {
    if let Ok(mut child) = child.lock() {
        let _ = child.kill();
    }
}

fn consensus_agents() -> [AgentSpec; 3] {
    [
        AgentSpec {
            name: "claude",
            program: "claude",
            args: &["-p"],
        },
        AgentSpec {
            name: "gemini",
            program: "gemini",
            args: &["--approval-mode", "plan"],
        },
        AgentSpec {
            name: "codex",
            program: "codex",
            args: &["exec", "-s", "read-only", "--skip-git-repo-check", "-"],
        },
    ]
}

#[derive(Clone, Copy)]
struct AgentSpec {
    name: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

struct ConsensusTempPaths {
    shared_prompt: PathBuf,
    synthesis_prompt: PathBuf,
}

impl ConsensusTempPaths {
    fn new(job_id: &str) -> Result<Self, String> {
        let dir = temp_dir()?;
        fs::create_dir_all(&dir)
            .map_err(|err| format!("failed to create consensus temp dir: {err}"))?;
        set_dir_permissions(&dir)?;
        let safe_job_id = safe_job_id(job_id);
        Ok(Self {
            shared_prompt: dir.join(format!("{safe_job_id}.consensus.prompt")),
            synthesis_prompt: dir.join(format!("{safe_job_id}.consensus.synth.prompt")),
        })
    }

    fn cleanup(&self) {
        let _ = fs::remove_file(&self.shared_prompt);
        let _ = fs::remove_file(&self.synthesis_prompt);
    }
}

fn temp_dir() -> Result<PathBuf, String> {
    let mut dir = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("aura-desktop");
    dir.push("tmp");
    Ok(dir)
}

fn write_private_file(path: &Path, content: &str) -> Result<(), String> {
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
fn set_dir_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|err| format!("failed to set consensus temp dir permissions: {err}"))
}

#[cfg(not(unix))]
fn set_dir_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn safe_job_id(job_id: &str) -> String {
    job_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>()
}

fn send_event(on_event: &Channel<AiEvent>, event: AiEvent) -> Result<(), String> {
    on_event
        .send(event)
        .map_err(|err| format!("failed to send AI event: {err}"))
}
