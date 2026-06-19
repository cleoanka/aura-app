use crate::agent::AgentAuth;
use crate::exec::{AiEvent, JobHandle, JobRegistry};
use crate::{agent_manager, env_resolver};
use command_group::{CommandGroup, GroupChild};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdout, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::ipc::Channel;

const TOTAL_TIMEOUT: Duration = Duration::from_secs(420);
// Asılan/yavaş bir ajan tüm consensus'u kilitlemesin: 420s (7dk) çok uzundu → 90s.
// Bir model normalde 15-60s'de yanıtlar; 90s aşılırsa o ajan düşürülür, kalanlarla devam.
const AGENT_TIMEOUT: Duration = Duration::from_secs(90);
const SYNTHESIS_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusAnswerMode {
    NoAnswers,
    SingleAgent,
    Synthesize,
}

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

pub fn pick_synthesizer<'a>(available: &'a [&'a str]) -> Option<&'a str> {
    ["claude", "gemini", "codex"]
        .into_iter()
        .find(|candidate| available.iter().any(|agent| agent == candidate))
}

pub fn consensus_answer_mode(answer_count: usize) -> ConsensusAnswerMode {
    match answer_count {
        0 => ConsensusAnswerMode::NoAnswers,
        1 => ConsensusAnswerMode::SingleAgent,
        _ => ConsensusAnswerMode::Synthesize,
    }
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
    send_event(&on_event, AiEvent::Job { job_id: job_id.clone() })?;

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
    let available_agents = tokio::task::spawn_blocking(detect_usable_agents)
        .await
        .unwrap_or_else(|_| fallback_usable_agents());

    if !available_agents.is_empty() {
        let names = available_agents
            .iter()
            .map(|agent| agent.name)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = on_event.send(AiEvent::Status {
            text: format!(
                "🔵 {} ajana paralel soruluyor: {names}…",
                available_agents.len()
            ),
            stage: Some("consensus".to_string()),
            agent: None,
        });
    }

    let total = available_agents.len();
    let mut set = tokio::task::JoinSet::new();
    for agent in available_agents.iter().copied() {
        set.spawn(run_agent(
            agent,
            shared_prompt_path.clone(),
            on_event.clone(),
            handle.clone(),
        ));
    }

    // Tamamlanma SIRASINA göre topla (ajanlar paralel çalışır) → her biten anında görünür,
    // hızlı ajan yavaş/asılan ajanı beklerken UI donmuş gibi durmaz. Asılan ajan run_agent'in
    // 90s timeout'unda kendi child'ını öldürüp düşer; consensus kalan yanıtlarla devam eder.
    let mut answers = Vec::new();
    let mut done = 0usize;
    while let Some(res) = set.join_next().await {
        done += 1;
        if let Ok(Some((agent, answer))) = res {
            if !answer.trim().is_empty() {
                answers.push((agent, answer));
            }
        }
        let remaining = total.saturating_sub(done);
        if remaining > 0 && !handle.is_cancelled() {
            let _ = on_event.send(AiEvent::Status {
                text: format!("⏳ {done}/{total} ajan yanıtladı · {remaining} bekleniyor…"),
                stage: Some("consensus".to_string()),
                agent: None,
            });
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
        let reason = "no AI agents available".to_string();
        send_event(
            &on_event,
            AiEvent::Error {
                reason: reason.clone(),
                taxonomy: "model".to_string(),
            },
        )?;
        return Err(reason);
    }

    match consensus_answer_mode(answers.len()) {
        ConsensusAnswerMode::NoAnswers => {
            let reason = "no AI agents available".to_string();
            send_event(
                &on_event,
                AiEvent::Error {
                    reason: reason.clone(),
                    taxonomy: "model".to_string(),
                },
            )?;
            return Err(reason);
        }
        ConsensusAnswerMode::SingleAgent => {
            let (agent, answer) = answers
                .into_iter()
                .next()
                .ok_or_else(|| "no AI agents available".to_string())?;
            return stream_single_agent_answer(&agent, &answer, on_event);
        }
        ConsensusAnswerMode::Synthesize => {}
    }

    let synthesis_prompt = synth_prompt(&query, &answers);
    write_private_file(&synthesis_prompt_path, &synthesis_prompt)?;
    let available_names = available_agents
        .iter()
        .map(|agent| agent.name)
        .collect::<Vec<_>>();
    match pick_synthesizer(&available_names) {
        Some(synthesizer) => {
            let agent = available_agents
                .iter()
                .find(|agent| agent.name == synthesizer)
                .copied()
                .ok_or_else(|| format!("selected synthesizer {synthesizer} was not available"))?;
            let _ = on_event.send(AiEvent::Status {
                text: format!("🧠 {synthesizer} yanıtları sentezliyor…"),
                stage: Some("synthesize".to_string()),
                agent: Some(synthesizer.to_string()),
            });
            match run_synthesis(
                agent,
                &synthesis_prompt_path,
                on_event.clone(),
                handle.clone(),
            )
            .await
            {
                Ok(text) => Ok(text),
                Err(reason) if handle.is_cancelled() => {
                    send_event(
                        &on_event,
                        AiEvent::Error {
                            reason: reason.clone(),
                            taxonomy: "cancelled".to_string(),
                        },
                    )?;
                    Err(reason)
                }
                Err(_) => stream_concatenated_answers(&answers, on_event),
            }
        }
        None => stream_concatenated_answers(&answers, on_event),
    }
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

    let _ = on_event.send(AiEvent::Status {
        text: format!("✓ {} yanıtladı", agent.name),
        stage: Some("consensus".to_string()),
        agent: Some(agent.name.to_string()),
    });
    Some((agent.name.to_string(), output))
}

async fn run_synthesis(
    agent: AgentSpec,
    prompt_path: &Path,
    on_event: Channel<AiEvent>,
    handle: JobHandle,
) -> Result<String, String> {
    if handle.is_cancelled() {
        return Err("consensus job cancelled".to_string());
    }

    let child = spawn_agent(&agent, prompt_path, &handle)?;
    let stdout = {
        let mut child = child
            .lock()
            .map_err(|_| format!("failed to lock {} synthesis process", agent.name))?;
        child
            .inner()
            .stdout
            .take()
            .ok_or_else(|| format!("failed to capture {} synthesis stdout", agent.name))?
    };

    let read_task = tokio::task::spawn_blocking({
        let on_event = on_event.clone();
        let cancelled = handle.cancelled();
        move || stream_stdout(stdout, on_event, cancelled)
    });

    let read_result = match tokio::time::timeout(SYNTHESIS_TIMEOUT, read_task).await {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => Err(format!(
            "{} synthesis reader task failed: {err}",
            agent.name
        )),
        Err(_) => {
            kill_child(&child);
            Err(format!("{} synthesis timed out", agent.name))
        }
    };

    let status = wait_child(child).await;

    match (read_result, status) {
        (Ok(text), Some(status)) if status.success() => {
            send_event(&on_event, AiEvent::Done { run_dir: None })?;
            Ok(text)
        }
        (Ok(_), Some(status)) => {
            let reason = format!("{} synthesis exited with status {status}", agent.name);
            Err(reason)
        }
        (Ok(_), None) => {
            let reason = format!("failed to wait for {} synthesis", agent.name);
            Err(reason)
        }
        (Err(reason), _) => Err(reason),
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

/// OOM koruması (audit #4): kaçak/anormal ajan GB'larca stdout üretirse belleği şişirmesin.
/// exec.rs::read_jsonl ile aynı 16MB sınırı.
const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

fn read_stdout_to_string(stdout: ChildStdout) -> Result<String, String> {
    let mut text = String::new();
    BufReader::new(stdout)
        .take(MAX_OUTPUT_BYTES as u64)
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
    // .take() TOPLAM okunan baytı sınırlar → tek bir devasa satır bile OOM yapamaz (codex).
    for line in BufReader::new(stdout.take(MAX_OUTPUT_BYTES as u64)).lines() {
        let mut line = line.map_err(|err| format!("failed to read synthesis output: {err}"))?;
        line.push('\n');
        if text.len() + line.len() > MAX_OUTPUT_BYTES {
            break;
        }
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

fn stream_single_agent_answer(
    agent: &str,
    answer: &str,
    on_event: Channel<AiEvent>,
) -> Result<String, String> {
    let text = format!("({agent}: tek ajan / single agent)\n{answer}");
    send_event(&on_event, AiEvent::Chunk { text: text.clone() })?;
    send_event(&on_event, AiEvent::Done { run_dir: None })?;
    Ok(text)
}

fn stream_concatenated_answers(
    answers: &[(String, String)],
    on_event: Channel<AiEvent>,
) -> Result<String, String> {
    let mut text = String::new();
    for (agent, answer) in answers {
        text.push_str("\n## ");
        text.push_str(agent);
        text.push('\n');
        text.push_str(answer.trim());
        text.push('\n');
    }
    let text = text.trim_start().to_string();
    send_event(&on_event, AiEvent::Chunk { text: text.clone() })?;
    send_event(&on_event, AiEvent::Done { run_dir: None })?;
    Ok(text)
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

fn detect_usable_agents() -> Vec<AgentSpec> {
    let all_agents = consensus_agents();
    match agent_manager::detect(false) {
        Ok(report) => all_agents
            .into_iter()
            .filter(|agent| {
                report
                    .agents
                    .get(agent.name)
                    .map(|status| {
                        status.installed
                            && !matches!(status.auth, AgentAuth::LoggedOut)
                            && status.can_invoke != Some(false)
                    })
                    .unwrap_or_else(|| command_available(agent.program))
            })
            .collect(),
        Err(_) => fallback_usable_agents(),
    }
}

fn fallback_usable_agents() -> Vec<AgentSpec> {
    consensus_agents()
        .into_iter()
        .filter(|agent| command_available(agent.program))
        .collect()
}

fn command_available(program: &str) -> bool {
    env_resolver::login_command("/bin/zsh")
        .args(["-lc", &format!("command -v {program}")])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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
