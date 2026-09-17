//! Application lifecycle: startup and configuration reloads.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{self, ConfigPaths, ConfigSnapshot, Diagnostic};
use crate::desktop::{hotkey, tray, window};
use crate::error::{AppError, AppResult};
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

    app.manage(AppState::new(paths, config, http));

    register_hotkey(app);
    tray::create(app)?;
    config::watcher::start(app)?;

    if show_on_startup {
        window::show(app)?;
    }
    Ok(())
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
