//! Tools implemented in Rust, referenced from manifests via `{"type": "builtin", "id": "…"}`.

use rust_i18n::t;
use serde::Deserialize;
use tauri::Manager;

use super::args::Args;
use super::executor::{ExecContext, Success, ToolDisplay};
use crate::integrations::media::MediaAction;
use crate::integrations::radio;
use crate::integrations::spotify::{self, SearchKind};
use crate::launcher::{FAST_PATH_MIN_SCORE, TOOL_MIN_SCORE};
use crate::search;
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
    Radio,
    WebSearch,
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
            Builtin::Radio => "radio",
            Builtin::WebSearch => "web_search",
        }
    }

    /// Parameters the manifest must declare for this builtin.
    pub fn required_params(self) -> &'static [&'static str] {
        match self {
            Builtin::ClipboardWrite => &["text"],
            Builtin::OpenApp => &["name"],
            Builtin::SpotifyPlay | Builtin::WebSearch => &["query"],
            Builtin::MediaControl => &["action"],
            Builtin::Radio => &["device", "state"],
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
    /// when an installed app matches confidently, `radio` only for known devices ("schalte das
    /// Licht an" is not ours); otherwise the model gets a chance.
    pub fn accepts_fast_path(self, ctx: &ExecContext<'_>, args: &Args) -> bool {
        match self {
            Builtin::Radio => {
                radio::Device::parse(str_arg(args, "device")).is_ok()
                    && radio::Command::parse(str_arg(args, "state")).is_ok()
            }
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

    pub async fn run(self, ctx: &ExecContext<'_>, args: &Args) -> Result<Success, String> {
        let text = match self {
            Builtin::WebSearch => return web_search(ctx, str_arg(args, "query")).await,
            Builtin::Datetime => Ok(chrono::Local::now().format("%A, %Y-%m-%d %H:%M:%S (%:z)").to_string()),
            Builtin::ClipboardRead => {
                blocking(|| {
                    let text = clipboard()?
                        .get_text()
                        .map_err(|e| t!("clipboard.no_text", error = e).to_string())?;
                    Ok(if text.is_empty() {
                        t!("clipboard.empty").into()
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
                        .map_err(|e| t!("clipboard.write_failed", error = e).to_string())?;
                    Ok(t!("clipboard.copied").into())
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
            Builtin::MediaControl => {
                let action = MediaAction::parse(str_arg(args, "action"))?;
                let level = args.get("level").and_then(|v| v.as_i64());
                let amount = args.get("amount").and_then(|v| v.as_i64());
                // Core Audio is COM: keep it off the async runtime threads.
                blocking(move || action.run(level, amount)).await
            }
            Builtin::Radio => {
                let device = radio::Device::parse(str_arg(args, "device"))?;
                let command = radio::Command::parse(str_arg(args, "state"))?;
                blocking(move || radio::run(device, command)).await
            }
        };
        text.map(Success::from)
    }
}

async fn web_search(ctx: &ExecContext<'_>, query: &str) -> Result<Success, String> {
    let config = ctx.app.state::<AppState>().config();
    let response = search::search(ctx.http, &config.settings.search, query).await?;
    Ok(Success {
        content: response.for_model(query),
        display: Some(ToolDisplay::WebResults {
            query: query.trim().to_string(),
            results: response.results,
        }),
    })
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
            Ok(t!("launcher.launched", name = app.name).into())
        }
        None => {
            let suggestions: Vec<String> = launcher.search(name, 3).into_iter().map(|(_, a)| a.name).collect();
            Err(if suggestions.is_empty() {
                t!("launcher.not_found", name = name).into()
            } else {
                t!("launcher.ambiguous", name = name, suggestions = suggestions.join(", ")).into()
            })
        }
    }
}

fn str_arg<'a>(args: &'a Args, name: &str) -> &'a str {
    args.get(name).and_then(|v| v.as_str()).unwrap_or_default()
}

fn clipboard() -> Result<arboard::Clipboard, String> {
    arboard::Clipboard::new().map_err(|e| t!("clipboard.unavailable", error = e).to_string())
}

async fn blocking<F>(f: F) -> Result<String, String>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("internal error: {e}"))?
}
