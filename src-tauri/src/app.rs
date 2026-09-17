//! Application lifecycle: startup, background warm-up and configuration reloads.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{self, ConfigPaths, ConfigSnapshot, Diagnostic};
use crate::desktop::{hotkey, tray, window};
use crate::error::{AppError, AppResult};
use crate::launcher::AppIndex;
use crate::llm::OllamaClient;
use crate::secrets;
use crate::state::AppState;

pub const EVENT_CONFIG_CHANGED: &str = "orbit://config-changed";

pub fn setup(app: &AppHandle) -> AppResult<()> {
    let paths = ConfigPaths::resolve(app)?;
    paths.ensure_initialized()?;
    log::info!("config folder: {}", paths.root.display());

    let config = config::load(&paths, None);
    let show_on_startup = config.settings.window.show_on_startup;

    let http = reqwest::Client::builder()
        .user_agent(concat!("Orbit/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::msg(format!("HTTP-Client: {e}")))?;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::msg(format!("Cache-Ordner nicht gefunden: {e}")))?;
    let launcher = AppIndex::load(cache_dir.join("app-index.json"));

    app.manage(AppState::new(paths, config, http, launcher));

    register_hotkey(app);
    tray::create(app)?;
    config::watcher::start(app)?;
    warm_up(app);

    if show_on_startup {
        window::show(app)?;
    }
    Ok(())
}

/// Prepares everything that would otherwise slow down the first request, in the background:
/// the default model is loaded into memory, the app index refreshed and the Spotify token fetched.
fn warm_up(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<AppState>();
        tokio::join!(
            preload_profile(&state, None),
            async {
                if let Err(e) = state.launcher.refresh().await {
                    log::warn!("launcher refresh: {e}");
                }
            },
            state.spotify.warm_up(&state.http),
        );
    });
}

/// Loads a profile's model into memory so the first answer starts without delay.
pub async fn preload_profile(state: &AppState, profile_id: Option<&str>) {
    let config = state.config();
    let settings = &config.settings;
    let profile = settings.profile(profile_id);
    let client = OllamaClient::new(state.http.clone(), &settings.ollama.base_url);
    match client.preload(&profile.model, &settings.ollama.keep_alive).await {
        Ok(()) => log::info!("model preloaded: {}", profile.model),
        Err(e) => log::warn!("model preload {}: {e}", profile.model),
    }
}

/// Reloads configuration from disk, applies runtime changes and notifies the frontend.
pub fn reload_config(app: &AppHandle) -> ConfigSnapshot {
    let state = app.state::<AppState>();
    let previous = state.config();
    let next = config::load(&state.paths, Some(&previous.settings));
    let hotkey_changed = previous.settings.hotkey != next.settings.hotkey;
    state.replace_config(next);

    if hotkey_changed {
        register_hotkey(app);
    }

    let snapshot = snapshot(app);
    if let Err(e) = app.emit(EVENT_CONFIG_CHANGED, &snapshot) {
        log::error!("emit config change: {e}");
    }
    snapshot
}

pub fn snapshot(app: &AppHandle) -> ConfigSnapshot {
    let state = app.state::<AppState>();
    let hotkey_problem = state
        .hotkey_error()
        .map(|message| Diagnostic::new("settings.json", message));
    ConfigSnapshot::build(&state.config(), state.paths.root.display().to_string(), hotkey_problem)
}

/// A secret required by at least one tool. The value itself is never exposed.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretInfo {
    pub name: String,
    pub label: String,
    pub is_set: bool,
    pub used_by: Vec<String>,
}

pub fn secret_overview(app: &AppHandle) -> Vec<SecretInfo> {
    let state = app.state::<AppState>();
    state
        .config()
        .tools
        .secret_usage()
        .into_iter()
        .map(|(name, used_by)| SecretInfo {
            label: secrets::label(&name),
            is_set: matches!(secrets::get(&name), Ok(Some(_))),
            name,
            used_by,
        })
        .collect()
}

/// Stores (`Some`) or deletes (`None`) a secret and refreshes dependent integrations.
pub async fn update_secret(state: &AppState, name: &str, value: Option<&str>) -> AppResult<()> {
    match value {
        Some(value) => secrets::set(name, value),
        None => secrets::delete(name),
    }
    .map_err(AppError::msg)?;
    log::info!("secret {} {}", name, if value.is_some() { "saved" } else { "deleted" });

    if name.starts_with("spotify_") {
        state.spotify.invalidate().await;
        state.spotify.warm_up(&state.http).await;
    }
    Ok(())
}

pub fn open_config_folder(app: &AppHandle) -> AppResult<()> {
    let root = app.state::<AppState>().paths.root.clone();
    app.opener()
        .open_path(root.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::msg(format!("Ordner konnte nicht geöffnet werden: {e}")))
}

fn register_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    let result = hotkey::register(app, &state.config().settings.hotkey);
    if let Err(message) = &result {
        log::error!("{message}");
    }
    state.set_hotkey_error(result.err());
}
