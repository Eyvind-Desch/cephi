//! IPC commands invoked from the frontend. Kept thin: all logic lives in the modules.
//!
//! Mirrored in `src/lib/ipc.ts`.

use rust_i18n::t;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::agent::{self, AgentEvent, Input};
use crate::config::ConfigSnapshot;
use crate::desktop::web::{self, WebState};
use crate::desktop::window::{self, WindowMode};
use crate::error::{AppError, AppResult};
use crate::secrets::SecretGroupInfo;
use crate::state::AppState;
use crate::translate::Translation;

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
    agent::run(&app, &state, Input::Message(text), profile_id, on_event).await
}

/// Lets the model answer from the latest tool results (e.g. summarize search results).
#[tauri::command]
pub async fn agent_continue(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: Option<String>,
    on_event: Channel<AgentEvent>,
) -> AppResult<()> {
    agent::run(&app, &state, Input::Continue, profile_id, on_event).await
}

/// Loads the profile's model into memory in the background (e.g. after switching profiles).
#[tauri::command]
pub async fn agent_preload(state: State<'_, AppState>, profile_id: Option<String>) -> AppResult<()> {
    crate::app::preload_profile(&state, profile_id.as_deref()).await;
    Ok(())
}

#[tauri::command]
pub fn secrets_list(state: State<'_, AppState>) -> Vec<SecretGroupInfo> {
    crate::app::secret_overview(&state)
}

#[tauri::command]
pub async fn secrets_test(state: State<'_, AppState>, group_id: String) -> AppResult<String> {
    crate::app::test_secret_group(&state, &group_id).await
}

#[tauri::command]
pub async fn secrets_set(app: AppHandle, name: String, value: String) -> AppResult<()> {
    crate::app::update_secret(&app, &name, Some(&value)).await
}

#[tauri::command]
pub async fn secrets_delete(app: AppHandle, name: String) -> AppResult<()> {
    crate::app::update_secret(&app, &name, None).await
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

#[tauri::command]
pub fn web_state(state: State<'_, AppState>) -> WebState {
    state.web.state()
}

/// Address bar: opens a URL or domain, or searches for the input.
#[tauri::command]
pub async fn web_navigate(app: AppHandle, input: String) -> AppResult<()> {
    web::navigate_input(&app, &input).await
}

/// Opens a link (e.g. a search result) in the web tab and switches to it.
#[tauri::command]
pub async fn web_open(app: AppHandle, url: String) -> AppResult<()> {
    let parsed = url::Url::parse(&url).map_err(|_| AppError::msg(t!("external.invalid_link")))?;
    web::open(&app, parsed).await
}

/// Position and visibility of the page; `seq` orders reports that arrive out of order.
#[tauri::command]
pub fn web_set_layout(app: AppHandle, bounds: web::Bounds, visible: bool, seq: u64) -> AppResult<()> {
    web::set_layout(&app, bounds, visible, seq)
}

#[tauri::command]
pub fn web_command(app: AppHandle, command: web::WebCommand) -> AppResult<()> {
    web::command(&app, command)
}

/// Translator tab. `to`: language code or name, `None` = automatic direction.
#[tauri::command]
pub async fn translate(state: State<'_, AppState>, text: String, to: Option<String>) -> AppResult<Translation> {
    let config = state.config();
    crate::translate::translate(&state.http, &config.settings.translate, &text, to.as_deref())
        .await
        .map_err(AppError::msg)
}

/// Opens a link from a chat message in the default browser. Web URLs only.
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> AppResult<()> {
    let parsed = url::Url::parse(&url).map_err(|_| AppError::msg(t!("external.invalid_link")))?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err(AppError::msg(t!("external.web_only")));
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| AppError::msg(e.to_string()))
}
