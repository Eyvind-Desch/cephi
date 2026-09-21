//! The Keys page: which secrets a setup needs, and storing them in the Windows Credential Manager.
//!
//! Changing a key changes what works: cloud profiles become (un)available and MCP servers have to
//! start again, because a server reads its keys only once, when it starts.

use rust_i18n::t;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{AppError, AppResult};
use crate::llm::LlmClient;
use crate::secrets::{self, SecretGroupInfo};
use crate::state::AppState;

use super::{snapshot, EVENT_CONFIG_CHANGED};

pub fn secret_overview(state: &AppState) -> Vec<SecretGroupInfo> {
    let config = state.config();
    let mut usage = config.tools.secret_usage();
    // Keys referenced as {{secret:…}} in mcp.json show up next to the ones tools use.
    for (name, server) in &config.mcp.mcp_servers {
        for secret in server.secrets() {
            usage.entry(secret).or_default().push(format!("MCP · {name}"));
        }
    }
    secrets::overview(&usage, &config.settings)
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

    // MCP servers read their keys only when they start.
    let affected: Vec<String> = state
        .config()
        .mcp
        .mcp_servers
        .iter()
        .filter(|(_, server)| server.secrets().iter().any(|secret| secret == name))
        .map(|(server, _)| server.clone())
        .collect();
    if !affected.is_empty() {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            let config = state.config();
            state.mcp.restart(&affected, &config.mcp).await;
            if let Err(e) = app.emit(EVENT_CONFIG_CHANGED, snapshot(&app)) {
                log::error!("emit config change: {e}");
            }
        });
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
