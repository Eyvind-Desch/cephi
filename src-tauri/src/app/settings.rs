//! The Settings page writing to `settings.json` and `mcp.json`, so nobody has to edit JSON.
//!
//! Every change goes through [`crate::config::edit`], which keeps the files readable, and is then
//! applied like a change made in an editor: the config is reloaded and the UI gets the new snapshot.

use rust_i18n::t;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{self, ConfigSnapshot};
use crate::error::{AppError, AppResult};
use crate::mcp::oauth;
use crate::state::AppState;

use super::{reload_config, snapshot};

/// Merges `patch` into settings.json and applies it right away.
pub fn update_settings(app: &AppHandle, patch: &serde_json::Value) -> AppResult<ConfigSnapshot> {
    let state = app.state::<AppState>();
    config::edit::update_settings(&state.paths.settings, patch).map_err(AppError::msg)?;
    Ok(reload_config(app))
}

/// Adds or changes an MCP server in mcp.json.
pub fn save_mcp_server(
    app: &AppHandle,
    name: &str,
    previous: Option<&str>,
    server: &serde_json::Value,
) -> AppResult<ConfigSnapshot> {
    let state = app.state::<AppState>();
    config::edit::save_mcp_server(&state.paths.mcp, name, previous, server).map_err(AppError::msg)?;
    Ok(reload_config(app))
}

pub fn remove_mcp_server(app: &AppHandle, name: &str) -> AppResult<ConfigSnapshot> {
    // A deleted server keeps no login.
    oauth::forget(name);
    let state = app.state::<AppState>();
    config::edit::remove_mcp_server(&state.paths.mcp, name).map_err(AppError::msg)?;
    Ok(reload_config(app))
}

/// Browser login for an MCP server that uses OAuth; afterwards the server is (re)started.
pub async fn mcp_login(app: &AppHandle, server: &str) -> AppResult<ConfigSnapshot> {
    let state = app.state::<AppState>();
    let config = state.config();
    let url = config
        .mcp
        .mcp_servers
        .get(server)
        .and_then(|entry| entry.url.clone())
        .ok_or_else(|| AppError::msg(t!("mcp.no_url", server = server)))?;

    let opener = app.clone();
    oauth::login(&state.http, server, &url, move |auth_url| {
        opener
            .opener()
            .open_url(auth_url, None::<&str>)
            .map_err(|e| t!("mcp.login_failed", error = e).to_string())
    })
    .await
    .map_err(AppError::msg)?;

    state.mcp.restart(&[server.to_string()], &config.mcp).await;
    Ok(snapshot(app))
}

/// Forgets a server's browser login.
pub async fn mcp_logout(app: &AppHandle, server: &str) -> AppResult<ConfigSnapshot> {
    let state = app.state::<AppState>();
    oauth::forget(server);
    let config = state.config();
    state.mcp.restart(&[server.to_string()], &config.mcp).await;
    Ok(snapshot(app))
}
