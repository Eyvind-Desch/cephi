use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use rust_i18n::t;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::args::Args;
use super::manifest::{Action, HttpMethod, OpenTarget};
use super::registry::Tool;
use super::template::{self, Context, Secrets};
use crate::desktop::web;
use crate::search::WebResult;
use crate::translate::Translation;

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
    /// Structured data the UI renders richer than plain text.
    pub display: Option<ToolDisplay>,
}

/// Mirrored in `src/lib/ipc.ts` (`ToolDisplay`).
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ToolDisplay {
    WebResults {
        query: String,
        results: Vec<WebResult>,
    },
    /// A page opened in the web tab; the UI links back to it.
    WebPage {
        url: String,
    },
    Translation(Translation),
}

/// A successful tool run: text for the model plus optional display data.
pub struct Success {
    pub content: String,
    pub display: Option<ToolDisplay>,
}

impl From<String> for Success {
    fn from(content: String) -> Self {
        Self { content, display: None }
    }
}

impl ToolOutput {
    fn from_result(result: Result<Success, String>) -> Self {
        match result {
            Ok(success) => Self {
                ok: true,
                content: success.content,
                display: success.display,
            },
            Err(content) => Self::error(content),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            content: message.into(),
            display: None,
        }
    }
}

pub async fn execute(ctx: &ExecContext<'_>, tool: &Tool, args: &Args) -> ToolOutput {
    log::info!("tool {} ({})", tool.manifest.name, tool.manifest.action.kind());
    let result = match &tool.manifest.action {
        Action::OpenUrl { url, target } => open_url(ctx, url, *target, args).await,
        Action::Launch {
            program,
            args: argv,
            cwd,
        } => launch(program, argv, cwd.as_deref(), args).map(Success::from),
        Action::Powershell { script, timeout_secs } => powershell(script, args, *timeout_secs).await.map(Success::from),
        Action::Http {
            method,
            url,
            headers,
            body,
            timeout_secs,
        } => http(ctx, *method, url, headers, body.as_deref(), *timeout_secs, args)
            .await
            .map(Success::from),
        Action::Builtin { id } => id.run(ctx, args).await,
    };
    if let Err(e) = &result {
        log::warn!("tool {} failed: {e}", tool.manifest.name);
    }
    ToolOutput::from_result(result)
}

/// Whether a trigger match may run the tool directly, without the model.
pub fn accepts_fast_path(ctx: &ExecContext<'_>, tool: &Tool, args: &Args) -> bool {
    match &tool.manifest.action {
        Action::Builtin { id } => id.accepts_fast_path(ctx, args),
        _ => true,
    }
}

/// Human-readable description of what the tool is about to do, shown in the confirmation dialog.
/// Secrets are masked.
pub fn preview(tool: &Tool, args: &Args) -> String {
    let render = |text: &str, context| template::render(text, context, args, Secrets::Mask).unwrap_or_else(|e| e);
    match &tool.manifest.action {
        Action::OpenUrl { url, .. } => t!("executor.preview_open", url = render(url, Context::Url)).into(),
        Action::Launch {
            program, args: argv, ..
        } => {
            let rendered: Vec<String> = argv
                .iter()
                .map(|a| render(a, Context::Arg))
                .filter(|a| !a.is_empty())
                .collect();
            let command = format!("{} {}", render(program, Context::Arg), rendered.join(" "));
            t!("executor.preview_launch", command = command.trim_end()).into()
        }
        // Model-controlled values first (what the user actually has to judge), then the fixed script.
        Action::Powershell { script, .. } => {
            let values: Vec<String> = args.iter().map(|(k, v)| format!("{k}: {}", plain(v))).collect();
            if values.is_empty() {
                script.trim().to_string()
            } else {
                t!(
                    "executor.preview_script",
                    values = values.join("\n"),
                    script = script.trim()
                )
                .into()
            }
        }
        Action::Http { method, url, .. } => format!("{} {}", method.as_str(), render(url, Context::Url)),
        Action::Builtin { id } => t!("executor.preview_builtin", id = id.as_str()).into(),
    }
}

async fn open_url(
    ctx: &ExecContext<'_>,
    url_template: &str,
    target: OpenTarget,
    args: &Args,
) -> Result<Success, String> {
    let rendered = template::render(url_template, Context::Url, args, Secrets::Resolve)?;
    // Messages go to the model and the UI, so they use the masked URL.
    let shown = template::render(url_template, Context::Url, args, Secrets::Mask)?;
    let parsed =
        url::Url::parse(&rendered).map_err(|e| t!("executor.invalid_url", url = shown, error = e).to_string())?;

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
        return Err(t!("executor.scheme_not_allowed", scheme = parsed.scheme()).into());
    }

    if target == OpenTarget::Cephi {
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(t!("executor.scheme_not_allowed", scheme = parsed.scheme()).into());
        }
        let url = parsed.to_string();
        web::open(ctx.app, parsed)
            .await
            .map_err(|e| t!("executor.open_failed", error = e).to_string())?;
        return Ok(Success {
            content: t!("executor.opened_in_cephi", url = shown).into(),
            // A URL with secrets must not reach the UI.
            display: template::secret_names(url_template)
                .is_empty()
                .then_some(ToolDisplay::WebPage { url }),
        });
    }

    ctx.app
        .opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| t!("executor.open_failed", error = e).to_string())?;
    Ok(Success::from(t!("executor.opened", url = shown).to_string()))
}

fn launch(program: &str, argv: &[String], cwd: Option<&str>, args: &Args) -> Result<String, String> {
    let program = expand_env(&template::render(program, Context::Arg, args, Secrets::Resolve)?);
    let rendered: Vec<String> = argv
        .iter()
        .map(|a| template::render(a, Context::Arg, args, Secrets::Resolve).map(|a| expand_env(&a)))
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
        command.current_dir(expand_env(&template::render(
            cwd,
            Context::Arg,
            args,
            Secrets::Resolve,
        )?));
    }
    command
        .spawn()
        .map_err(|e| t!("executor.launch_failed", program = program, error = e).to_string())?;
    Ok(t!("executor.launched", program = program).into())
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
        let value = plain(value);
        command.env(env_var_name(name), &value);
        // Scripts written before the rename use the old prefix.
        command.env(legacy_env_var_name(name), value);
    }

    let child = command
        .spawn()
        .map_err(|e| t!("executor.powershell_start_failed", error = e).to_string())?;
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| t!("executor.timeout", seconds = timeout.as_secs()).to_string())?
        .map_err(|e| t!("executor.powershell_failed", error = e).to_string())?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(if stdout.is_empty() {
            t!("executor.no_output").into()
        } else {
            stdout
        })
    } else {
        let details = if stderr.is_empty() { stdout } else { stderr };
        Err(t!(
            "executor.exit_code",
            code = output.status.code().unwrap_or(-1),
            details = details
        )
        .into())
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
    let url = template::render(url_template, Context::Url, args, Secrets::Resolve)?;
    let mut request = match method {
        HttpMethod::Get => ctx.http.get(&url),
        HttpMethod::Post => ctx.http.post(&url),
    }
    .timeout(Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS)));

    for (name, value) in headers {
        request = request.header(name, template::render(value, Context::Header, args, Secrets::Resolve)?);
    }
    if let Some(body) = body {
        request = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(template::render(body, Context::Body, args, Secrets::Resolve)?);
    }

    // `without_url`: the URL may contain secrets and must not end up in messages.
    let response = request
        .send()
        .await
        .map_err(|e| t!("executor.request_failed", error = e.without_url()).to_string())?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| t!("executor.response_unreadable", error = e).to_string())?;
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!("HTTP {status}: {text}"))
    }
}

pub fn env_var_name(param: &str) -> String {
    format!("CEPHI_ARG_{}", param.to_ascii_uppercase())
}

/// Name used before the rename to Cephi; still set so existing tools keep working.
fn legacy_env_var_name(param: &str) -> String {
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
        assert_eq!(env_var_name("command"), "CEPHI_ARG_COMMAND");
        assert_eq!(legacy_env_var_name("command"), "ORBIT_ARG_COMMAND");
    }
}
