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
/// A redirect request is a single line; more than this is not one.
const MAX_REQUEST_BYTES: usize = 16_384;
/// Shown to the server during registration.
const CLIENT_NAME: &str = "Cephi";

/// Vault name of a server's token chunk. The fingerprint keeps servers apart whose names only
/// differ in characters the vault does not allow ("my-server" and "my_server"), and keeps a long
/// name from colliding with another one after being cut to length.
fn secret_name(server: &str, index: usize) -> String {
    let clean: String = server
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .take(30)
        .collect();
    format!("mcp_oauth_{clean}_{:08x}_p{index}", fingerprint(server))
}

/// FNV-1a over the server name; it only has to separate names, not to be cryptographic.
fn fingerprint(server: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in server.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
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

/// Brackets a piece: the vault trims whitespace, which would eat a space at a chunk border.
const FENCE: char = '#';

/// Reads a server's token pieces back into one string (empty: nothing stored).
fn read_chunks(server: &str) -> Result<String, String> {
    let mut json = String::new();
    for index in 0.. {
        match crate::secrets::get(&secret_name(server, index))? {
            // Exactly one fence per side: the value itself may contain that character.
            Some(part) => json.push_str(
                part.strip_prefix(FENCE)
                    .and_then(|p| p.strip_suffix(FENCE))
                    .unwrap_or(&part),
            ),
            None => break,
        }
    }
    Ok(json)
}

/// Stores a string in pieces of [`CHUNK_BYTES`]; old pieces are removed first.
fn write_chunks(server: &str, value: &str) -> Result<(), String> {
    forget(server);
    // Chunk on character boundaries: a split UTF-8 sequence would not survive the vault.
    let mut rest = value;
    let mut index = 0;
    while !rest.is_empty() {
        let mut end = rest.len().min(CHUNK_BYTES);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (chunk, remainder) = rest.split_at(end);
        crate::secrets::set(&secret_name(server, index), &format!("{FENCE}{chunk}{FENCE}"))?;
        rest = remainder;
        index += 1;
    }
    Ok(())
}

#[async_trait]
impl CredentialStore for VaultStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, rmcp::transport::auth::AuthError> {
        let json = read_chunks(&self.server).map_err(rmcp::transport::auth::AuthError::InternalError)?;
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
        write_chunks(&self.server, &json).map_err(rmcp::transport::auth::AuthError::InternalError)
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

        // The request may arrive in several pieces; the first line is all that matters.
        let mut raw = Vec::new();
        let mut buffer = [0u8; 2048];
        while !raw.contains(&b'\n') && raw.len() < MAX_REQUEST_BYTES {
            match stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(read) => raw.extend_from_slice(&buffer[..read]),
                Err(e) => return Err(t!("mcp.login_failed", error = e).to_string()),
            }
        }
        let request = String::from_utf8_lossy(&raw);
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
        assert!(first.starts_with("mcp_oauth_github_mcp_"), "{first}");
        assert!(crate::secrets::is_valid_name(&first));
        assert_ne!(secret_name("GitHub-MCP", 1), first, "chunks differ");
        assert_ne!(secret_name("notion", 0), first);
        // Names the vault cannot tell apart on their own stay separate through the fingerprint.
        assert_ne!(secret_name("my-server", 0), secret_name("my_server", 0));
        assert_ne!(secret_name("foo", 1), secret_name("foo_p1", 0));
        // The vault limits names to 64 characters.
        assert!(crate::secrets::is_valid_name(&secret_name(&"x".repeat(80), 9)));
    }

    /// Touches the real Credential Manager, like the other secret tests.
    #[test]
    fn long_tokens_survive_the_vault_in_pieces() {
        // The vault is a shared resource: parallel writes from other tests fail intermittently.
        let _serial = crate::secrets::CREDENTIAL_MANAGER
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let server = format!("test{}", uuid::Uuid::new_v4().simple());
        // Longer than one credential entry, with spaces exactly at the chunk borders.
        let token = format!(
            "{}a b{} c{}#x#",
            " ".repeat(CHUNK_BYTES - 1),
            " ".repeat(CHUNK_BYTES),
            "#".repeat(CHUNK_BYTES)
        );
        write_chunks(&server, &token).unwrap();
        assert!(is_logged_in(&server));
        assert_eq!(read_chunks(&server).unwrap(), token);

        // A shorter token must not leave older pieces behind.
        write_chunks(&server, "short").unwrap();
        assert_eq!(read_chunks(&server).unwrap(), "short");

        forget(&server);
        assert!(!is_logged_in(&server));
        assert_eq!(read_chunks(&server).unwrap(), "");
    }

    /// The browser sends the redirect to the loopback port; only that one carries the code.
    #[tokio::test]
    async fn the_callback_server_answers_the_browser_and_returns_the_url() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(wait_for_callback(listener, port));

        // A browser also asks for the favicon; it must not end the wait.
        for request in [
            "GET /favicon.ico HTTP/1.1

",
            "GET /callback?code=abc&state=xyz HTTP/1.1

",
        ] {
            let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut answer = String::new();
            stream.read_to_string(&mut answer).await.unwrap();
            assert!(answer.starts_with("HTTP/1.1 200 OK"), "{answer}");
        }

        let callback = server.await.unwrap().unwrap();
        assert_eq!(callback, format!("http://127.0.0.1:{port}/callback?code=abc&state=xyz"));
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
