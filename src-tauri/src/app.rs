//! Application lifecycle: startup, background warm-up and configuration reloads.

use std::time::Duration;

use rust_i18n::t;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{self, ConfigPaths, ConfigSnapshot, Diagnostic};
use crate::desktop::{autostart, hotkey, tray, window};
use crate::error::{AppError, AppResult};
use crate::history::ChatStore;
use crate::launcher::AppIndex;
use crate::migrate;
use crate::llm::{LlmClient, OllamaClient, LOCAL_PROVIDER_ID};
use crate::secrets::{self, SecretGroupInfo};
use crate::state::AppState;

pub const EVENT_CONFIG_CHANGED: &str = "cephi://config-changed";

pub fn setup(app: &AppHandle) -> AppResult<()> {
    let paths = ConfigPaths::resolve(app)?;
    if !ConfigPaths::is_overridden() {
        migrate::from_orbit(app, &paths.root);
    }
    paths.ensure_initialized()?;
    log::info!("config folder: {}", paths.root.display());

    let config = config::load(&paths, None);
    // Started with Windows: wait in the tray until the hotkey is pressed.
    let autostarted = std::env::args().any(|arg| arg == autostart::AUTOSTART_ARG);
    let show_on_startup = config.settings.window.show_on_startup && !autostarted;

    let http = reqwest::Client::builder()
        .user_agent(concat!("Cephi/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::msg(format!("failed to create HTTP client: {e}")))?;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::msg(format!("cache directory not found: {e}")))?;
    let launcher = AppIndex::load(cache_dir.join("app-index.json"));

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::msg(format!("data directory not found: {e}")))?;
    let chats = ChatStore::new(data_dir.join("chats"));
    let saved_history = chats.load().map(|chat| chat.history);

    app.manage(AppState::new(paths, config, http, launcher, chats));
    if let Some(history) = saved_history {
        log::info!("chat restored: {} messages", history.len());
        app.state::<AppState>().agent.restore(history);
    }

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

/// Loads a local profile's model into memory so the first answer starts without delay.
/// Cloud profiles need no warm-up.
pub async fn preload_profile(state: &AppState, profile_id: Option<&str>) {
    let config = state.config();
    let settings = &config.settings;
    let profile = settings.profile(profile_id);
    if profile.provider != LOCAL_PROVIDER_ID {
        return;
    }
    let client = OllamaClient::new(state.http.clone(), &settings.ollama);
    match client.preload(&profile.model).await {
        Ok(()) => log::info!("model preloaded: {}", profile.model),
        Err(e) => log::warn!("model preload {}: {e}", profile.model),
    }
}

/// Reloads configuration from disk, applies runtime changes and notifies the frontend.
pub fn reload_config(app: &AppHandle) -> ConfigSnapshot {
    let state = app.state::<AppState>();
    let previous = state.config();
    let next = config::load(&state.paths, Some(&previous.settings));
    let hotkey_changed = previous.settings.hotkey != next.settings.hotkey
        || previous.settings.selection_hotkey != next.settings.selection_hotkey;
    let language_changed = previous.settings.language != next.settings.language;
    state.replace_config(next);

    if hotkey_changed || language_changed {
        // Also re-registers on language change so a hotkey error message is re-localized.
        register_hotkey(app);
    }
    if language_changed {
        if let Err(e) = tray::refresh_menu(app) {
            log::error!("refresh tray menu: {e}");
        }
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

pub fn secret_overview(state: &AppState) -> Vec<SecretGroupInfo> {
    let config = state.config();
    secrets::overview(&config.tools.secret_usage(), &config.settings)
}

/// Stores (`Some`) or deletes (`None`) a secret and refreshes dependent integrations.
pub async fn update_secret(app: &AppHandle, name: &str, value: Option<&str>) -> AppResult<()> {
    let state = app.state::<AppState>();
    match value {
        Some(value) => secrets::set(name, value),
        None => secrets::delete(name),
    }
    .map_err(AppError::msg)?;
    log::info!("secret {} {}", name, if value.is_some() { "saved" } else { "deleted" });

    // Cloud profiles become (un)available when their key changes.
    if let Err(e) = app.emit(EVENT_CONFIG_CHANGED, &snapshot(app)) {
        log::error!("emit config change: {e}");
    }

    if name.starts_with("spotify_") {
        state.spotify.invalidate().await;
        state.spotify.warm_up(&state.http).await;
    }
    Ok(())
}

/// Verifies the stored keys of a service with a real request. Returns a success message.
pub async fn test_secret_group(state: &AppState, group_id: &str) -> AppResult<String> {
    let config = state.config();
    if group_id == "spotify" {
        return state.spotify.test_connection(&state.http).await.map_err(AppError::msg);
    }
    let provider = config
        .settings
        .provider(group_id)
        .ok_or_else(|| AppError::msg(format!("no connection test for \"{group_id}\"")))?;
    let client = LlmClient::for_provider(&config.settings, &provider.id, &state.http)
        .map_err(|e| AppError::msg(e.to_string()))?;
    client.check().await.map_err(|e| AppError::msg(e.to_string()))?;
    Ok(t!("llm.connection_ok", provider = provider.display_name()).into())
}

pub fn open_config_folder(app: &AppHandle) -> AppResult<()> {
    let root = app.state::<AppState>().paths.root.clone();
    app.opener()
        .open_path(root.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::msg(t!("app.open_folder_failed", error = e)))
}

fn register_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = &state.config().settings;
    let result = hotkey::register(app, &settings.hotkey, settings.selection_hotkey.as_deref());
    if let Err(message) = &result {
        log::error!("{message}");
    }
    state.set_hotkey_error(result.err());
}

/// The saved chat as the frontend renders it (`None` = start empty).
pub fn load_chat(state: &AppState) -> Option<serde_json::Value> {
    state.chats.load().map(|chat| chat.ui)
}

/// Saves the conversation: the frontend's messages together with the model history.
pub async fn save_chat(state: &AppState, ui: serde_json::Value) -> AppResult<()> {
    let history = state.agent.history().await;
    state.chats.save(ui, &history).map_err(AppError::from)
}

/// "New chat": archives the current one and clears the model history.
pub async fn new_chat(state: &AppState) -> AppResult<()> {
    state.agent.reset().await;
    state.chats.archive().map_err(AppError::from)
}
