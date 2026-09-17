//! IPC commands invoked from the frontend. Kept thin: all logic lives in the modules.
//!
//! Mirrored in `src/lib/ipc.ts`.

use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::agent::{self, AgentEvent};
use crate::app::SecretInfo;
use crate::config::ConfigSnapshot;
use crate::desktop::window::{self, WindowMode};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
pub fn config_snapshot(app: AppHandle) -> ConfigSnapshot {
    crate::app::snapshot(&app)
}

#[tauri::command]
pub fn config_reload(app: AppHandle) -> ConfigSnapshot {
    crate::app::reload_config(&app)
}

#[tauri::command]
pub fn config_open_folder(app: AppHandle) -> AppResult<()> {
    crate::app::open_config_folder(&app)
}

#[tauri::command]
pub async fn agent_send(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    profile_id: Option<String>,
    on_event: Channel<AgentEvent>,
) -> AppResult<()> {
    agent::run(&app, &state, text, profile_id, on_event).await
}

/// Loads the profile's model into memory in the background (e.g. after switching profiles).
#[tauri::command]
pub async fn agent_preload(state: State<'_, AppState>, profile_id: Option<String>) -> AppResult<()> {
    crate::app::preload_profile(&state, profile_id.as_deref()).await;
    Ok(())
}

#[tauri::command]
pub fn secrets_list(app: AppHandle) -> Vec<SecretInfo> {
    crate::app::secret_overview(&app)
}

#[tauri::command]
pub async fn secrets_set(state: State<'_, AppState>, name: String, value: String) -> AppResult<()> {
    crate::app::update_secret(&state, &name, Some(&value)).await
}

#[tauri::command]
pub async fn secrets_delete(state: State<'_, AppState>, name: String) -> AppResult<()> {
    crate::app::update_secret(&state, &name, None).await
}

#[tauri::command]
pub fn agent_cancel(state: State<'_, AppState>) {
    state.agent.cancel();
}

#[tauri::command]
pub fn agent_confirm(state: State<'_, AppState>, call_id: String, approved: bool) -> bool {
    state.agent.confirm(&call_id, approved)
}

#[tauri::command]
pub async fn agent_reset(state: State<'_, AppState>) -> AppResult<()> {
    state.agent.reset().await;
    Ok(())
}

#[tauri::command]
pub fn window_set_mode(app: AppHandle, mode: WindowMode) -> AppResult<()> {
    window::set_mode(&app, mode)
}

#[tauri::command]
pub fn window_hide(app: AppHandle) -> AppResult<()> {
    window::hide(&app)
}

#[tauri::command]
pub fn window_set_pinned(state: State<'_, AppState>, pinned: bool) {
    state.window.set_pinned(pinned);
}

/// Opens a link from a chat message in the default browser. Web URLs only.
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> AppResult<()> {
    let parsed = url::Url::parse(&url).map_err(|_| AppError::msg("Ungültiger Link."))?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err(AppError::msg("Nur Web-Links können geöffnet werden."));
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| AppError::msg(e.to_string()))
}
