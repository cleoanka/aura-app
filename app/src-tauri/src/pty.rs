use crate::env_resolver;
use portable_pty::{
    native_pty_system, Child, CommandBuilder, MasterPty, PtySize, PtySystem, SlavePty,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;

static SESSIONS: OnceLock<Mutex<HashMap<String, PtySession>>> = OnceLock::new();
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

struct PtySession {
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send>,
    master: Box<dyn MasterPty + Send>,
}

pub fn login_argv(agent: &str) -> Result<Vec<String>, String> {
    match agent {
        "claude" => Ok(vec!["claude".to_string(), "/login".to_string()]),
        "gemini" => Ok(vec!["gemini".to_string()]),
        "codex" => Ok(vec!["codex".to_string(), "login".to_string()]),
        _ => Err(format!("unsupported pty login agent: {agent}")),
    }
}

pub fn open(agent: &str, on_output: Channel<String>) -> Result<String, String> {
    let argv = login_argv(agent)?;
    let program = argv
        .first()
        .ok_or_else(|| "pty login argv is empty".to_string())?;

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("failed to open pty: {err}"))?;

    let mut command = CommandBuilder::new(program.clone());
    for arg in argv.iter().skip(1) {
        command.arg(arg.clone());
    }
    for (key, value) in env_resolver::login_env() {
        command.env(key.clone(), value.clone());
    }

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| format!("failed to start {program}: {err}"))?;
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| format!("failed to clone pty reader: {err}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|err| format!("failed to open pty writer: {err}"))?;
    let session_id = new_session_id();

    registry().lock().map_err(|err| err.to_string())?.insert(
        session_id.clone(),
        PtySession {
            writer,
            child,
            master: pair.master,
        },
    );

    let thread_session_id = session_id.clone();
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(len) => {
                    let chunk = String::from_utf8_lossy(&buffer[..len]).into_owned();
                    if on_output.send(chunk).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        let _ = on_output.send("\r\n[oturum bitti]\r\n".to_string());
        remove_and_kill(&thread_session_id);
    });

    Ok(session_id)
}

pub fn write(session_id: &str, data: &str) -> Result<(), String> {
    let mut sessions = registry().lock().map_err(|err| err.to_string())?;
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| format!("unknown pty session: {session_id}"))?;

    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|err| format!("failed to write to pty: {err}"))?;
    session
        .writer
        .flush()
        .map_err(|err| format!("failed to flush pty writer: {err}"))
}

pub fn resize(session_id: &str, rows: u16, cols: u16) -> Result<(), String> {
    if rows == 0 || cols == 0 {
        return Err("pty rows and cols must be greater than zero".to_string());
    }

    let sessions = registry().lock().map_err(|err| err.to_string())?;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| format!("unknown pty session: {session_id}"))?;

    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("failed to resize pty: {err}"))
}

pub fn close(session_id: &str) -> Result<(), String> {
    remove_and_kill(session_id).ok_or_else(|| format!("unknown pty session: {session_id}"))
}

fn registry() -> &'static Mutex<HashMap<String, PtySession>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remove_and_kill(session_id: &str) -> Option<()> {
    let mut session = registry().lock().ok()?.remove(session_id)?;
    let _ = session.child.kill();
    Some(())
}

fn new_session_id() -> String {
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    format!("pty-{}-{nanos:x}-{counter:x}", std::process::id())
}
