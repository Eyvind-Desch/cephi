//! The saved conversation behind the chat and the history list.
//!
//! A chat has two halves that always travel together: what the UI renders and what the model sees
//! (see [`crate::history`]). The frontend hands over its half as opaque JSON.

use crate::error::{AppError, AppResult};
use crate::state::AppState;

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

/// Opens an archived chat: the current one is archived, the model continues the old conversation.
pub async fn open_archived_chat(state: &AppState, id: &str) -> AppResult<serde_json::Value> {
    let chat = state.chats.open_archived(id).map_err(AppError::from)?;
    state.agent.reset().await;
    state.agent.restore(chat.history);
    Ok(chat.ui)
}
