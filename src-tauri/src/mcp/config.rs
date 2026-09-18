//! `mcp.json`: the MCP servers Cephi connects to.
//!
//! Same format as Claude Desktop (`{ "mcpServers": { "<name>": { "command", "args", "env" } } }`), so
//! existing snippets can be pasted as they are. Servers reached over HTTP use `url` (+ `headers`), as in
//! Cursor or VS Code. Unknown keys from other clients are ignored. Cephi additions: `disabled`,
//! `permission`, `autoApprove`, and `{{secret:name}}` in `env`/`headers` for keys from the vault.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use regex::Regex;
use rust_i18n::t;
use serde::{Deserialize, Serialize};

use crate::tools::Permission;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    #[serde(default)]
    pub mcp_servers: BTreeMap<String, ServerConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// Local server: program to start (e.g. `npx`, `uvx`, `docker`, a path).
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub cwd: Option<String>,
    /// Remote server (streamable HTTP).
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
    /// `confirm` (default): every call asks first. `auto`: calls run without asking.
    #[serde(default)]
    pub permission: Permission,
    /// Tools of this server that run without asking even when `permission` is `confirm`.
    #[serde(default)]
    pub auto_approve: Vec<String>,
}

pub enum Transport<'a> {
    Process { command: &'a str },
    Http { url: &'a str },
}

impl ServerConfig {
    pub fn transport(&self) -> Result<Transport<'_>, String> {
        match (self.command.as_deref(), self.url.as_deref()) {
            (Some(command), None) if !command.trim().is_empty() => Ok(Transport::Process { command }),
            (None, Some(url)) if url.starts_with("http://") || url.starts_with("https://") => {
                Ok(Transport::Http { url })
            }
            (Some(_), Some(_)) => Err(t!("mcp.command_and_url").into()),
            (None, Some(url)) => Err(t!("mcp.invalid_url", url = url).into()),
            _ => Err(t!("mcp.no_command").into()),
        }
    }

    /// Keys from the vault referenced as `{{secret:name}}` in `env` or `headers`.
    pub fn secrets(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .env
            .values()
            .chain(self.headers.values())
            .flat_map(|value| secret_refs(value))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Whether both start the same server (permissions aside, which apply without a restart).
    pub fn same_connection(&self, other: &ServerConfig) -> bool {
        (
            &self.command,
            &self.args,
            &self.env,
            &self.cwd,
            &self.url,
            &self.headers,
        ) == (
            &other.command,
            &other.args,
            &other.env,
            &other.cwd,
            &other.url,
            &other.headers,
        )
    }

    pub fn is_auto_approved(&self, tool: &str) -> bool {
        self.permission == Permission::Auto || self.auto_approve.iter().any(|t| t == tool)
    }
}

/// Reads `mcp.json`. A missing file means "no servers"; a broken one is reported and ignored.
pub fn load(path: &Path) -> Result<McpConfig, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(McpConfig::default()),
        Err(e) => return Err(t!("config.file_unreadable", error = e).into()),
    };
    let config: McpConfig =
        serde_json::from_str(&text).map_err(|e| t!("config.invalid_json", error = e).to_string())?;
    validate(&config)?;
    Ok(config)
}

/// Checks names and that every server has exactly one way to reach it.
pub fn validate(config: &McpConfig) -> Result<(), String> {
    for (name, server) in &config.mcp_servers {
        if !is_valid_server_name(name) {
            return Err(t!("mcp.invalid_name", name = name).into());
        }
        server.transport().map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

/// Replaces `{{secret:name}}` with the stored key. Fails with the name of a missing key.
pub fn resolve_secrets(value: &str) -> Result<String, String> {
    let mut missing = None;
    let resolved = secret_pattern().replace_all(value, |caps: &regex::Captures| {
        let name = &caps[1];
        match crate::secrets::get(name) {
            Ok(Some(secret)) => secret,
            _ => {
                missing.get_or_insert_with(|| name.to_string());
                String::new()
            }
        }
    });
    match missing {
        Some(name) => Err(t!("secrets.missing", name = name).into()),
        None => Ok(resolved.into_owned()),
    }
}

fn secret_refs(value: &str) -> Vec<String> {
    secret_pattern()
        .captures_iter(value)
        .map(|caps| caps[1].to_string())
        .collect()
}

fn secret_pattern() -> Regex {
    Regex::new(r"\{\{\s*secret:([a-z][a-z0-9_]{1,63})\s*\}\}").expect("valid regex")
}

/// Server names become part of tool names: letters, digits, `-` and `_`.
fn is_valid_server_name(name: &str) -> bool {
    (1..=40).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> McpConfig {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn reads_claude_desktop_and_http_servers() {
        let config = parse(
            r#"{ "mcpServers": {
                "filesystem": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\Users"] },
                "github": { "url": "https://api.githubcopilot.com/mcp/", "headers": { "Authorization": "Bearer {{secret:github_token}}" },
                            "autoApprove": ["search_code"], "someOtherClientKey": 1 }
            } }"#,
        );
        let fs = &config.mcp_servers["filesystem"];
        assert!(matches!(fs.transport(), Ok(Transport::Process { command: "npx" })));
        assert_eq!(fs.permission, Permission::Confirm);

        let github = &config.mcp_servers["github"];
        assert!(matches!(github.transport(), Ok(Transport::Http { .. })));
        assert_eq!(github.secrets(), vec!["github_token"]);
        assert!(github.is_auto_approved("search_code"));
        assert!(!github.is_auto_approved("create_issue"));
    }

    #[test]
    fn permissions_do_not_count_as_a_different_connection() {
        let server = ServerConfig {
            command: Some("npx".into()),
            ..Default::default()
        };
        let approved = ServerConfig {
            auto_approve: vec!["search".into()],
            ..server.clone()
        };
        assert!(server.same_connection(&approved));
        let other_args = ServerConfig {
            args: vec!["-y".into()],
            ..server.clone()
        };
        assert!(!server.same_connection(&other_args));
    }

    #[test]
    fn a_server_needs_exactly_one_of_command_and_url() {
        assert!(ServerConfig::default().transport().is_err());
        let both = ServerConfig {
            command: Some("npx".into()),
            url: Some("https://x.example".into()),
            ..Default::default()
        };
        assert!(both.transport().is_err());
    }

    #[test]
    fn missing_secrets_are_reported() {
        let err = resolve_secrets("Bearer {{secret:cephi_mcp_test_missing}}").unwrap_err();
        assert!(err.contains("cephi_mcp_test_missing"), "{err}");
        assert_eq!(resolve_secrets("plain").unwrap(), "plain");
    }

    #[test]
    fn server_names() {
        assert!(is_valid_server_name("brave-search"));
        assert!(!is_valid_server_name("with space"));
    }
}
