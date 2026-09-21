//! IPC commands invoked from the frontend. Kept thin: all logic lives in the modules.
//!
//! Mirrored in `src/lib/ipc.ts`.

use rust_i18n::t;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::agent::{self, AgentEvent, Input};
use crate::config::ConfigSnapshot;
use crate::desktop::autostart::{self, AutostartState};
use crate::desktop::web::{self, WebState};
use crate::desktop::window::{self, WindowMode};
use crate::error::{AppError, AppResult};
use crate::history::ArchivedChat;
use crate::desktop::capture::ScreenshotImage as Screenshot;
use crate::secrets::SecretGroupInfo;
use crate::services::{self, Service, ServiceStatus};
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

/// Settings page: changes keys in settings.json (objects merge, `null` removes).
#[tauri::command]
pub fn settings_update(app: AppHandle, patch: serde_json::Value) -> AppResult<ConfigSnapshot> {
    crate::app::update_settings(&app, &patch)
}

#[tauri::command]
pub fn mcp_save_server(
    app: AppHandle,
    name: String,
    previous_name: Option<String>,
    server: serde_json::Value,
) -> AppResult<ConfigSnapshot> {
    crate::app::save_mcp_server(&app, &name, previous_name.as_deref(), &server)
}

#[tauri::command]
pub fn mcp_remove_server(app: AppHandle, name: String) -> AppResult<ConfigSnapshot> {
    crate::app::remove_mcp_server(&app, &name)
}

#[tauri::command]
pub fn config_open_folder(app: AppHandle) -> AppResult<()> {
    crate::app::open_config_folder(&app)
}

#[tauri::command]
/// `images`: screenshots (PNG, base64) the question is about.
pub async fn agent_send(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    images: Option<Vec<String>>,
    profile_id: Option<String>,
    on_event: Channel<AgentEvent>,
) -> AppResult<()> {
    let input = Input::Message {
        text,
        images: images.unwrap_or_default(),
    };
    agent::run(&app, &state, input, profile_id, on_event).await
}

/// Takes a screenshot of the monitor the mouse is on (without Cephi itself) for a question about it.
#[tauri::command]
pub async fn screenshot_capture(app: AppHandle) -> AppResult<Screenshot> {
    crate::app::screenshot(&app).await
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

/// New chat: archives the current conversation.
#[tauri::command]
pub async fn agent_reset(state: State<'_, AppState>) -> AppResult<()> {
    crate::app::new_chat(&state).await
}

#[tauri::command]
pub fn chat_load(state: State<'_, AppState>) -> Option<serde_json::Value> {
    crate::app::load_chat(&state)
}

/// Saves the UI messages (after a request finished) together with the model history.
#[tauri::command]
pub async fn chat_save(state: State<'_, AppState>, ui: serde_json::Value) -> AppResult<()> {
    crate::app::save_chat(&state, ui).await
}

#[tauri::command]
pub fn chat_archive_list(state: State<'_, AppState>) -> Vec<ArchivedChat> {
    state.chats.list_archive()
}

/// Returns the UI messages of the opened chat.
#[tauri::command]
pub async fn chat_archive_open(state: State<'_, AppState>, id: String) -> AppResult<serde_json::Value> {
    crate::app::open_archived_chat(&state, &id).await
}

#[tauri::command]
pub fn chat_archive_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.chats.delete_archived(&id).map_err(AppError::from)
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
pub fn autostart_state() -> AutostartState {
    autostart::state()
}

#[tauri::command]
pub fn autostart_set(enabled: bool) -> AppResult<AutostartState> {
    autostart::set(enabled).map_err(AppError::msg)
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

/// Are the local helper services (web search, translator) running?
#[tauri::command]
pub async fn services_status(state: State<'_, AppState>) -> AppResult<Vec<ServiceStatus>> {
    let config = state.config();
    Ok(services::status(&state.http, &config.settings).await)
}

/// Starts a helper service (Docker in WSL) and waits until it answers.
#[tauri::command]
pub async fn service_start(state: State<'_, AppState>, service: Service) -> AppResult<()> {
    let config = state.config();
    services::start(&state.http, &config.settings, service)
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
