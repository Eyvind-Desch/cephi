//! Browser login for MCP servers that ask for OAuth instead of an API key.
//!
//! Cephi opens the server's login page in the browser and waits on a loopback port for the
//! redirect. The tokens land in the Windows Credential Manager like every other secret, never in
//! `mcp.json`; the client registers itself with the server (Dynamic Client Registration), so there
//! is nothing to set up by hand.

use std::time::Duration;

use async_trait::async_trait;
use rmcp::transport::auth::{
    AuthClient, AuthorizationManager, AuthorizationRequest, CredentialStore, OAuthState, StoredCredentials,
};
use rust_i18n::t;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// How long the browser has to finish the login.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
/// A credential entry holds at most 2560 bytes, tokens can be longer: they are stored in pieces.
const CHUNK_BYTES: usize = 2000;
/// Shown to the server during registration.
const CLIENT_NAME: &str = "Cephi";

/// Vault name of a server's first token chunk (the rest get `_2`, `_3`, …).
fn secret_name(server: &str, index: usize) -> String {
    let clean: String = server
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let base = format!("mcp_oauth_{clean}");
    let base: String = base.chars().take(56).collect();
    if index == 0 {
        base
    } else {
        format!("{base}_{}", index + 1)
    }
}

/// Whether a server has been logged in before.
pub fn is_logged_in(server: &str) -> bool {
    crate::secrets::is_set(&secret_name(server, 0))
}

/// Removes a server's tokens (logout, or when the server is deleted).
pub fn forget(server: &str) {
    for index in 0.. {
        let name = secret_name(server, index);
        if !crate::secrets::is_set(&name) {
            break;
        }
        if let Err(e) = crate::secrets::delete(&name) {
            log::warn!("mcp {server}: could not remove the login: {e}");
            break;
        }
    }
}

/// Keeps the OAuth tokens in the Windows Credential Manager, split into entries of [`CHUNK_BYTES`].
struct VaultStore {
    server: String,
}

#[async_trait]
impl CredentialStore for VaultStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, rmcp::transport::auth::AuthError> {
        let mut json = String::new();
        for index in 0.. {
            match crate::secrets::get(&secret_name(&self.server, index)) {
                Ok(Some(part)) => json.push_str(&part),
                Ok(None) => break,
                Err(e) => return Err(rmcp::transport::auth::AuthError::InternalError(e)),
            }
        }
        if json.is_empty() {
            return Ok(None);
        }
        match serde_json::from_str(&json) {
            Ok(credentials) => Ok(Some(credentials)),
            Err(e) => {
                log::warn!("mcp {}: stored login unreadable ({e}), logging in again", self.server);
                Ok(None)
            }
        }
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), rmcp::transport::auth::AuthError> {
        let json = serde_json::to_string(&credentials)
            .map_err(|e| rmcp::transport::auth::AuthError::InternalError(e.to_string()))?;
        // Old, possibly longer credentials must not leave pieces behind.
        forget(&self.server);
        for (index, chunk) in json.as_bytes().chunks(CHUNK_BYTES).enumerate() {
            let part = String::from_utf8_lossy(chunk).to_string();
            crate::secrets::set(&secret_name(&self.server, index), &part)
                .map_err(rmcp::transport::auth::AuthError::InternalError)?;
        }
        Ok(())
    }

    async fn clear(&self) -> Result<(), rmcp::transport::auth::AuthError> {
        forget(&self.server);
        Ok(())
    }
}

/// An HTTP client that authenticates as the logged-in user, or `None` without a stored login.
pub async fn client(
    http: &reqwest::Client,
    server: &str,
    url: &str,
) -> Result<Option<AuthClient<reqwest::Client>>, String> {
    if !is_logged_in(server) {
        return Ok(None);
    }
    let mut manager = AuthorizationManager::new(url).await.map_err(auth_error)?;
    manager.with_client(http.clone()).map_err(auth_error)?;
    manager.set_credential_store(VaultStore {
        server: server.to_string(),
    });
    if !manager.initialize_from_store().await.map_err(auth_error)? {
        return Ok(None);
    }
    Ok(Some(AuthClient::new(http.clone(), manager)))
}

/// Runs the login: opens `open_url` in the browser and waits for the redirect.
pub async fn login(
    http: &reqwest::Client,
    server: &str,
    url: &str,
    open_url: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    // Port 0: the system picks a free one, which the client registers as its redirect URI.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| t!("mcp.login_failed", error = e).to_string())?;
    let port = listener
        .local_addr()
        .map_err(|e| t!("mcp.login_failed", error = e).to_string())?
        .port();
    let redirect = format!("http://127.0.0.1:{port}/callback");

    let mut state = OAuthState::new(url, Some(http.clone())).await.map_err(auth_error)?;
    if let OAuthState::Unauthorized(manager) = &mut state {
        manager.set_credential_store(VaultStore {
            server: server.to_string(),
        });
    }

    let request = AuthorizationRequest::new(&redirect)
        .with_client_name(CLIENT_NAME)
        .with_application_type("native");
    state.start_authorization(request).await.map_err(auth_error)?;

    let OAuthState::Session(session) = &state else {
        return Err(t!("mcp.login_failed", error = "no authorization session").into());
    };
    open_url(session.get_authorization_url())?;

    let callback = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_callback(listener, port))
        .await
        .map_err(|_| t!("mcp.login_timeout").to_string())??;

    state.handle_callback_url(&callback).await.map_err(auth_error)?;
    state.complete_authorization().await.map_err(auth_error)?;
    log::info!("mcp {server}: logged in");
    Ok(())
}

/// Waits for the browser's redirect and answers it with a short page.
async fn wait_for_callback(listener: TcpListener, port: u16) -> Result<String, String> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|e| t!("mcp.login_failed", error = e).to_string())?;

        let mut buffer = vec![0u8; 8192];
        let read = stream
            .read(&mut buffer)
            .await
            .map_err(|e| t!("mcp.login_failed", error = e).to_string())?;
        let request = String::from_utf8_lossy(&buffer[..read]);
        let Some(path) = request_path(&request) else { continue };

        let body = t!("mcp.login_done").to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;

        // Browsers also ask for /favicon.ico; only the redirect carries the code.
        if path.contains("code=") || path.contains("error=") {
            return Ok(format!("http://127.0.0.1:{port}{path}"));
        }
    }
}

/// The path (with query) of a raw HTTP request.
fn request_path(request: &str) -> Option<&str> {
    let line = request.lines().next()?;
    let mut parts = line.split(' ');
    let method = parts.next()?;
    let path = parts.next()?;
    (method == "GET").then_some(path)
}

fn auth_error(error: rmcp::transport::auth::AuthError) -> String {
    t!("mcp.login_failed", error = error).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_names_are_valid_and_unique_per_server() {
        let first = secret_name("GitHub-MCP", 0);
        assert_eq!(first, "mcp_oauth_github_mcp");
        assert!(crate::secrets::is_valid_name(&first));
        assert_eq!(secret_name("GitHub-MCP", 1), "mcp_oauth_github_mcp_2");
        assert_ne!(secret_name("notion", 0), first);
        // The vault limits names to 64 characters.
        assert!(crate::secrets::is_valid_name(&secret_name(&"x".repeat(80), 9)));
    }

    #[test]
    fn only_get_requests_with_a_code_count() {
        assert_eq!(
            request_path("GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: x\r\n\r\n"),
            Some("/callback?code=abc&state=xyz")
        );
        assert_eq!(request_path("POST /callback HTTP/1.1"), None);
        assert_eq!(request_path(""), None);
    }
}
