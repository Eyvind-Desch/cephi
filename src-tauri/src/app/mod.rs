//! What the app does for the UI, between the IPC commands ([`crate::commands`]) and the modules
//! that do the work. This file holds the lifecycle — startup, background warm-up and configuration
//! reloads — and the submodules one page each.

mod chats;
mod keys;
mod media;
mod settings;

pub use chats::{load_chat, new_chat, open_archived_chat, save_chat};
pub use keys::{secret_overview, test_secret_group, update_secret};
pub use media::{dictate, screenshot, screenshot_region};
pub use settings::{mcp_login, mcp_logout, remove_mcp_server, save_mcp_server, update_settings};

use std::time::Duration;

use rust_i18n::t;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{self, ConfigPaths, ConfigSnapshot, Diagnostic};
use crate::desktop::{autostart, hotkey, region, tray, window};
use crate::error::{AppError, AppResult};
use crate::history::ChatStore;
use crate::launcher::AppIndex;
use crate::llm::{OllamaClient, LOCAL_PROVIDER_ID};
use crate::migrate;
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
    app.manage(region::Pending::default());
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
    sync_mcp(app);
}

/// Connects to the MCP servers from `mcp.json` in the background and tells the UI when their tools
/// are ready (starting a server can take a while, e.g. a first `npx` download).
fn sync_mcp(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let config = state.config();
        state.mcp.sync(&config.mcp).await;
        if let Err(e) = app.emit(EVENT_CONFIG_CHANGED, snapshot(&app)) {
            log::error!("emit config change: {e}");
        }
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

    // Unchanged servers keep running; see `McpManager::sync`.
    sync_mcp(app);

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
    ConfigSnapshot::build(
        &state.config(),
        &state.mcp,
        state.paths.root.display().to_string(),
        hotkey_problem,
    )
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
