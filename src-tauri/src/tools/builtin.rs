//! Tools implemented in Rust, referenced from manifests via `{"type": "builtin", "id": "…"}`.

use serde::Deserialize;
use tauri::Manager;

use super::args::Args;
use super::executor::ExecContext;
use crate::integrations::media::MediaAction;
use crate::integrations::spotify::{self, SearchKind};
use crate::launcher::{FAST_PATH_MIN_SCORE, TOOL_MIN_SCORE};
use crate::state::AppState;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    Datetime,
    ClipboardRead,
    ClipboardWrite,
    OpenApp,
    SpotifyPlay,
    MediaControl,
}

impl Builtin {
    pub fn as_str(self) -> &'static str {
        match self {
            Builtin::Datetime => "datetime",
            Builtin::ClipboardRead => "clipboard_read",
            Builtin::ClipboardWrite => "clipboard_write",
            Builtin::OpenApp => "open_app",
            Builtin::SpotifyPlay => "spotify_play",
            Builtin::MediaControl => "media_control",
        }
    }

    /// Parameters the manifest must declare for this builtin.
    pub fn required_params(self) -> &'static [&'static str] {
        match self {
            Builtin::ClipboardWrite => &["text"],
            Builtin::OpenApp => &["name"],
            Builtin::SpotifyPlay => &["query"],
            Builtin::MediaControl => &["action"],
            Builtin::Datetime | Builtin::ClipboardRead => &[],
        }
    }

    pub fn required_secrets(self) -> &'static [&'static str] {
        match self {
            Builtin::SpotifyPlay => &[spotify::CLIENT_ID_SECRET, spotify::CLIENT_SECRET_SECRET],
            _ => &[],
        }
    }

    /// Whether a trigger match may run this builtin without the model. `open_app` only does so
    /// when an installed app matches confidently; otherwise the model gets a chance.
    pub fn accepts_fast_path(self, ctx: &ExecContext<'_>, args: &Args) -> bool {
        match self {
            Builtin::OpenApp => {
                let state = ctx.app.state::<AppState>();
                state
                    .launcher
                    .best_match(str_arg(args, "name"), FAST_PATH_MIN_SCORE)
                    .is_some()
            }
            _ => true,
        }
    }

    pub async fn run(self, ctx: &ExecContext<'_>, args: &Args) -> Result<String, String> {
        match self {
            Builtin::Datetime => Ok(chrono::Local::now().format("%A, %Y-%m-%d %H:%M:%S (%:z)").to_string()),
            Builtin::ClipboardRead => {
                blocking(|| {
                    let text = clipboard()?
                        .get_text()
                        .map_err(|e| format!("Zwischenablage enthält keinen Text: {e}"))?;
                    Ok(if text.is_empty() {
                        "(Zwischenablage ist leer)".into()
                    } else {
                        text
                    })
                })
                .await
            }
            Builtin::ClipboardWrite => {
                let text = str_arg(args, "text").to_string();
                blocking(move || {
                    clipboard()?
                        .set_text(text)
                        .map_err(|e| format!("Schreiben fehlgeschlagen: {e}"))?;
                    Ok("Text in die Zwischenablage kopiert.".into())
                })
                .await
            }
            Builtin::OpenApp => open_app(ctx, str_arg(args, "name")).await,
            Builtin::SpotifyPlay => {
                let kind = SearchKind::parse(args.get("kind").and_then(|v| v.as_str()))?;
                let state = ctx.app.state::<AppState>();
                state
                    .spotify
                    .play(ctx.app, ctx.http, str_arg(args, "query"), kind)
                    .await
            }
            Builtin::MediaControl => Ok(MediaAction::parse(str_arg(args, "action"))?.run()),
        }
    }
}

async fn open_app(ctx: &ExecContext<'_>, name: &str) -> Result<String, String> {
    let state = ctx.app.state::<AppState>();
    let launcher = &state.launcher;

    let mut app = launcher.best_match(name, TOOL_MIN_SCORE);
    if app.is_none() {
        // Maybe the app was installed after the last index refresh.
        launcher.refresh().await?;
        app = launcher.best_match(name, TOOL_MIN_SCORE);
    }

    match app {
        Some(app) => {
            launcher.launch(&app)?;
            Ok(format!("Gestartet: {}", app.name))
        }
        None => {
            let suggestions: Vec<String> = launcher.search(name, 3).into_iter().map(|(_, a)| a.name).collect();
            Err(if suggestions.is_empty() {
                format!("Keine installierte App passt zu \"{name}\".")
            } else {
                format!(
                    "Keine App passt eindeutig zu \"{name}\". Ähnlich: {}.",
                    suggestions.join(", ")
                )
            })
        }
    }
}

fn str_arg<'a>(args: &'a Args, name: &str) -> &'a str {
    args.get(name).and_then(|v| v.as_str()).unwrap_or_default()
}

fn clipboard() -> Result<arboard::Clipboard, String> {
    arboard::Clipboard::new().map_err(|e| format!("Zwischenablage nicht verfügbar: {e}"))
}

async fn blocking<F>(f: F) -> Result<String, String>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Interner Fehler: {e}"))?
}
