use crate::agent::DoctorReport;
use crate::env_resolver::login_command;
use crate::error::{AppError, ErrorTaxonomy};

pub fn detect(probe: bool) -> Result<DoctorReport, AppError> {
    let doctor_command = if probe {
        "aura doctor --json --probe"
    } else {
        "aura doctor --json --no-probe"
    };

    let output = login_command("/bin/zsh")
        .args(["-lc", doctor_command])
        .output()
        .map_err(|error| AppError {
            taxonomy: ErrorTaxonomy::Sidecar,
            detail: format!("aura doctor başlatılamadı: {error}"),
            log_path: None,
        })?;

    if !output.status.success() {
        let detail = combined_output(&output.stdout, &output.stderr);
        let taxonomy = if output.status.code() == Some(127)
            || detail.contains("command not found")
            || detail.contains("not found: aura")
        {
            ErrorTaxonomy::Config
        } else {
            ErrorTaxonomy::Sidecar
        };

        return Err(AppError {
            taxonomy,
            detail: if detail.is_empty() {
                format!("aura doctor çıkış kodu: {}", output.status)
            } else {
                detail
            },
            log_path: None,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<DoctorReport>(&stdout).map_err(|error| AppError {
        taxonomy: ErrorTaxonomy::Sidecar,
        detail: format!("doctor JSON çözümlenemedi: {error}"),
        log_path: None,
    })
}

pub fn install_recipe(agent: &str) -> Option<&'static [&'static str]> {
    match agent {
        "claude" => Some(&["npm", "i", "-g", "@anthropic-ai/claude-code"]),
        "codex" => Some(&["npm", "i", "-g", "@openai/codex"]),
        // agy (Antigravity CLI): npm paketi değil + OAuth gerekir → otomatik install reçetesi yok.
        _ => None,
    }
}

pub fn install(agent: &str) -> Result<String, AppError> {
    let recipe = install_recipe(agent).ok_or_else(|| AppError {
        taxonomy: ErrorTaxonomy::Config,
        detail: format!("Bilinmeyen agent: {agent}"),
        log_path: None,
    })?;

    let output = login_command(recipe[0])
        .args(&recipe[1..])
        .output()
        .map_err(|error| AppError {
            taxonomy: ErrorTaxonomy::Config,
            detail: format!("Kurulum başlatılamadı: {error}"),
            log_path: None,
        })?;

    let detail = combined_output(&output.stdout, &output.stderr);

    if output.status.success() {
        Ok(detail)
    } else {
        Err(AppError {
            taxonomy: ErrorTaxonomy::Config,
            detail: if detail.is_empty() {
                format!("Kurulum çıkış kodu: {}", output.status)
            } else {
                detail
            },
            log_path: None,
        })
    }
}

fn combined_output(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);

    match (stdout.trim(), stderr.trim()) {
        ("", "") => String::new(),
        (stdout, "") => stdout.to_string(),
        ("", stderr) => stderr.to_string(),
        (stdout, stderr) => format!("{stdout}\n{stderr}"),
    }
}
