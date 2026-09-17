use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::args::Args;
use super::manifest::{Action, HttpMethod};
use super::registry::Tool;
use super::template::{self, Context};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Hides the console window of child processes.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct ExecContext<'a> {
    pub app: &'a AppHandle,
    pub http: &'a reqwest::Client,
}

/// Result passed back to the model (and shown in the UI).
#[derive(Debug)]
pub struct ToolOutput {
    pub ok: bool,
    pub content: String,
}

impl ToolOutput {
    fn from_result(result: Result<String, String>) -> Self {
        match result {
            Ok(content) => Self { ok: true, content },
            Err(content) => Self::error(content),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            content: message.into(),
        }
    }
}

pub async fn execute(ctx: &ExecContext<'_>, tool: &Tool, args: &Args) -> ToolOutput {
    log::info!("tool {} ({})", tool.manifest.name, tool.manifest.action.kind());
    let result = match &tool.manifest.action {
        Action::OpenUrl { url } => open_url(ctx, url, args),
        Action::Launch {
            program,
            args: argv,
            cwd,
        } => launch(program, argv, cwd.as_deref(), args),
        Action::Powershell { script, timeout_secs } => powershell(script, args, *timeout_secs).await,
        Action::Http {
            method,
            url,
            headers,
            body,
            timeout_secs,
        } => http(ctx, *method, url, headers, body.as_deref(), *timeout_secs, args).await,
        Action::Builtin { id } => id.run(args).await,
    };
    if let Err(e) = &result {
        log::warn!("tool {} failed: {e}", tool.manifest.name);
    }
    ToolOutput::from_result(result)
}

/// Human-readable description of what the tool is about to do, shown in the confirmation dialog.
pub fn preview(tool: &Tool, args: &Args) -> String {
    let render = |text: &str, context| template::render(text, context, args).unwrap_or_else(|e| e);
    match &tool.manifest.action {
        Action::OpenUrl { url } => format!("Öffnen: {}", render(url, Context::Url)),
        Action::Launch {
            program, args: argv, ..
        } => {
            let rendered: Vec<String> = argv
                .iter()
                .map(|a| render(a, Context::Arg))
                .filter(|a| !a.is_empty())
                .collect();
            format!("Starten: {} {}", render(program, Context::Arg), rendered.join(" "))
                .trim_end()
                .to_string()
        }
        // Model-controlled values first (what the user actually has to judge), then the fixed script.
        Action::Powershell { script, .. } => {
            let values: Vec<String> = args.iter().map(|(k, v)| format!("{k}: {}", plain(v))).collect();
            if values.is_empty() {
                script.trim().to_string()
            } else {
                format!("{}\n\nSkript:\n{}", values.join("\n"), script.trim())
            }
        }
        Action::Http { method, url, .. } => format!("{} {}", method.as_str(), render(url, Context::Url)),
        Action::Builtin { id } => format!("Interne Funktion: {}", id.as_str()),
    }
}

fn open_url(ctx: &ExecContext<'_>, url_template: &str, args: &Args) -> Result<String, String> {
    let rendered = template::render(url_template, Context::Url, args)?;
    let parsed = url::Url::parse(&rendered).map_err(|e| format!("Ungültige URL \"{rendered}\": {e}"))?;

    // A fixed scheme in the template (e.g. "ms-settings:") is trusted. Otherwise the model
    // controls the scheme, so only web URLs are allowed (no file:, javascript:, …).
    let fixed_scheme = template::literal_prefix(url_template)
        .split_once(':')
        .map(|(scheme, _)| scheme.to_ascii_lowercase());
    let allowed = match fixed_scheme {
        Some(scheme) => parsed.scheme() == scheme,
        None => matches!(parsed.scheme(), "http" | "https"),
    };
    if !allowed {
        return Err(format!(
            "Schema \"{}:\" ist für dieses Tool nicht erlaubt.",
            parsed.scheme()
        ));
    }

    ctx.app
        .opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| format!("Konnte nicht geöffnet werden: {e}"))?;
    Ok(format!("Geöffnet: {parsed}"))
}

fn launch(program: &str, argv: &[String], cwd: Option<&str>, args: &Args) -> Result<String, String> {
    let program = expand_env(&template::render(program, Context::Arg, args)?);
    let rendered: Vec<String> = argv
        .iter()
        .map(|a| template::render(a, Context::Arg, args).map(|a| expand_env(&a)))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|a| !a.is_empty())
        .collect();

    let mut command = std::process::Command::new(&program);
    command
        .args(&rendered)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(cwd) = cwd {
        command.current_dir(expand_env(&template::render(cwd, Context::Arg, args)?));
    }
    command
        .spawn()
        .map_err(|e| format!("\"{program}\" konnte nicht gestartet werden: {e}"))?;
    Ok(format!("Gestartet: {program}"))
}

async fn powershell(script: &str, args: &Args, timeout_secs: Option<u64>) -> Result<String, String> {
    // Force UTF-8 so umlauts survive the pipe.
    let script =
        format!("[Console]::OutputEncoding = [Text.Encoding]::UTF8\n$ErrorActionPreference = 'Stop'\n{script}");

    let mut command = tokio::process::Command::new(powershell_exe());
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true);
    for (name, value) in args {
        command.env(env_var_name(name), plain(value));
    }

    let child = command
        .spawn()
        .map_err(|e| format!("PowerShell konnte nicht gestartet werden: {e}"))?;
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| format!("Zeitüberschreitung nach {} s.", timeout.as_secs()))?
        .map_err(|e| format!("PowerShell-Fehler: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(if stdout.is_empty() {
            "(keine Ausgabe)".into()
        } else {
            stdout
        })
    } else {
        Err(format!(
            "Exit-Code {}:\n{}",
            output.status.code().unwrap_or(-1),
            if stderr.is_empty() { stdout } else { stderr }
        ))
    }
}

#[allow(clippy::too_many_arguments)]
async fn http(
    ctx: &ExecContext<'_>,
    method: HttpMethod,
    url_template: &str,
    headers: &std::collections::BTreeMap<String, String>,
    body: Option<&str>,
    timeout_secs: Option<u64>,
    args: &Args,
) -> Result<String, String> {
    let url = template::render(url_template, Context::Url, args)?;
    let mut request = match method {
        HttpMethod::Get => ctx.http.get(&url),
        HttpMethod::Post => ctx.http.post(&url),
    }
    .timeout(Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS)));

    for (name, value) in headers {
        request = request.header(name, template::render(value, Context::Header, args)?);
    }
    if let Some(body) = body {
        request = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(template::render(body, Context::Body, args)?);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("Anfrage fehlgeschlagen: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Antwort nicht lesbar: {e}"))?;
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!("HTTP {status}: {text}"))
    }
}

pub fn env_var_name(param: &str) -> String {
    format!("ORBIT_ARG_{}", param.to_ascii_uppercase())
}

fn plain(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Expands `%VAR%` references, e.g. `%LOCALAPPDATA%\Programs\…`.
fn expand_env(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Prefers PowerShell 7 (`pwsh`) and falls back to Windows PowerShell.
fn powershell_exe() -> &'static PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        std::env::var_os("PATH")
            .and_then(|paths| {
                std::env::split_paths(&paths)
                    .map(|dir| dir.join("pwsh.exe"))
                    .find(|p| p.is_file())
            })
            .unwrap_or_else(|| PathBuf::from("powershell.exe"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_known_env_vars_only() {
        std::env::set_var("ORBIT_TEST_VAR", "C:\\X");
        assert_eq!(expand_env("%ORBIT_TEST_VAR%\\a"), "C:\\X\\a");
        assert_eq!(expand_env("%ORBIT_MISSING%\\a"), "%ORBIT_MISSING%\\a");
        assert_eq!(expand_env("100% sicher"), "100% sicher");
    }

    #[test]
    fn env_var_names() {
        assert_eq!(env_var_name("command"), "ORBIT_ARG_COMMAND");
    }
}
