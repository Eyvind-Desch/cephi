//! MCP client: starts the servers from `mcp.json`, collects their tools and forwards calls.
//!
//! Every server is a separate connection (a child process or an HTTP session). Their tools are offered
//! to the model next to Cephi's own tools under the name `<server>__<tool>`. Calls ask for confirmation
//! unless the server or tool is approved in `mcp.json`.

pub mod config;
pub mod oauth;

use std::collections::{BTreeMap, HashMap};
use std::process::Stdio;
use std::sync::RwLock;
use std::time::Duration;

use rmcp::model::{CallToolRequestParams, ContentBlock};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{StreamableHttpClientTransport, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt};
use rust_i18n::t;
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::llm::ToolDef;
use config::{resolve_secrets, McpConfig, ServerConfig, Transport};

/// Includes a first `npx` download of the server package.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(90);
const CALL_TIMEOUT: Duration = Duration::from_secs(120);
/// Separates server and tool in the name the model sees.
const SEPARATOR: &str = "__";
/// Model APIs accept tool names up to 64 characters.
const MAX_TOOL_NAME: usize = 64;
/// Hides the console window of server processes.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A tool offered by an MCP server.
#[derive(Clone, Debug)]
pub struct McpTool {
    pub server: String,
    /// Name on the server.
    pub name: String,
    /// Name for the model: `<server>__<tool>`.
    pub exposed: String,
    pub title: String,
    pub description: String,
    pub input_schema: Value,
    pub auto_approved: bool,
}

/// Mirrored in `src/lib/ipc.ts` (`McpServerStatus`).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub name: String,
    pub state: ServerState,
    pub error: Option<String>,
    pub tools: usize,
    /// Whether a browser login (OAuth) is stored for this server.
    pub logged_in: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    Starting,
    Running,
    Failed,
    Disabled,
}

struct Connection {
    config: ServerConfig,
    client: RunningService<RoleClient, ()>,
    tools: Vec<McpTool>,
}

#[derive(Default)]
struct Catalog {
    tools: Vec<McpTool>,
    servers: Vec<ServerStatus>,
}

#[derive(Default)]
pub struct McpManager {
    connections: tokio::sync::Mutex<BTreeMap<String, Connection>>,
    /// Kept separately so the tool list can be read without waiting for a server that is starting.
    catalog: RwLock<Catalog>,
    /// Shared client for online servers (connection pool, proxy settings, user agent).
    http: reqwest::Client,
}

impl McpManager {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            ..Default::default()
        }
    }

    pub fn tools(&self) -> Vec<McpTool> {
        self.catalog.read().expect("mcp catalog lock poisoned").tools.clone()
    }

    pub fn servers(&self) -> Vec<ServerStatus> {
        self.catalog.read().expect("mcp catalog lock poisoned").servers.clone()
    }

    pub fn find(&self, exposed: &str) -> Option<McpTool> {
        self.catalog
            .read()
            .expect("mcp catalog lock poisoned")
            .tools
            .iter()
            .find(|tool| tool.exposed == exposed)
            .cloned()
    }

    /// What the model may call, each with the server it comes from (see `tools::select`).
    pub fn offered(&self) -> Vec<(ToolDef, String)> {
        self.tools()
            .into_iter()
            .map(|tool| {
                let server = tool.server;
                (
                    ToolDef {
                        name: tool.exposed,
                        description: tool.description,
                        parameters: tool.input_schema,
                    },
                    server,
                )
            })
            .collect()
    }

    /// Brings the running servers in line with `mcp.json`: stops removed or changed ones,
    /// (re)starts new, changed and previously failed ones. Unchanged servers keep running.
    pub async fn sync(&self, config: &McpConfig) {
        let mut connections = self.connections.lock().await;

        // Changed permissions apply to the running server; no restart needed.
        for (name, connection) in connections.iter_mut() {
            if let Some(wanted) = config.mcp_servers.get(name) {
                if wanted.same_connection(&connection.config) && *wanted != connection.config {
                    for tool in &mut connection.tools {
                        tool.auto_approved = wanted.is_auto_approved(&tool.name);
                    }
                    connection.config = wanted.clone();
                }
            }
        }

        let stale: Vec<String> = connections
            .iter()
            .filter(|(name, connection)| {
                config
                    .mcp_servers
                    .get(*name)
                    .is_none_or(|wanted| wanted.disabled || !wanted.same_connection(&connection.config))
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in stale {
            if let Some(connection) = connections.remove(&name) {
                log::info!("mcp {name}: stopping");
                let _ = connection.client.cancel().await;
            }
        }

        let to_start: Vec<(String, ServerConfig)> = config
            .mcp_servers
            .iter()
            .filter(|(name, server)| !server.disabled && !connections.contains_key(*name))
            .map(|(name, server)| (name.clone(), server.clone()))
            .collect();

        let mut failures: HashMap<String, String> = HashMap::new();
        if !to_start.is_empty() {
            self.publish(
                config,
                &connections,
                &failures,
                to_start.iter().map(|(n, _)| n.as_str()).collect(),
            );
            let started = futures_util::future::join_all(to_start.into_iter().map(|(name, server)| {
                let http = self.http.clone();
                async move {
                    let result = tokio::time::timeout(CONNECT_TIMEOUT, connect(&name, &server, &http))
                        .await
                        .unwrap_or_else(|_| Err(t!("mcp.timeout").into()));
                    (name, result)
                }
            }))
            .await;
            for (name, result) in started {
                match result {
                    Ok(connection) => {
                        log::info!("mcp {name}: {} tool(s)", connection.tools.len());
                        connections.insert(name, connection);
                    }
                    Err(error) => {
                        log::warn!("mcp {name}: {error}");
                        failures.insert(name, error);
                    }
                }
            }
        }
        self.publish(config, &connections, &failures, Vec::new());
    }

    /// Stops the named servers so the following [`sync`](Self::sync) starts them afresh (e.g. with a new key).
    pub async fn restart(&self, names: &[String], config: &McpConfig) {
        {
            let mut connections = self.connections.lock().await;
            for name in names {
                if let Some(connection) = connections.remove(name) {
                    log::info!("mcp {name}: restarting");
                    let _ = connection.client.cancel().await;
                }
            }
        }
        self.sync(config).await;
    }

    /// Stops all servers (on exit).
    pub async fn shutdown(&self) {
        let mut connections = self.connections.lock().await;
        for (_, connection) in std::mem::take(&mut *connections) {
            let _ = connection.client.cancel().await;
        }
    }

    pub async fn call(&self, tool: &McpTool, arguments: Value) -> Result<String, String> {
        let connections = self.connections.lock().await;
        let connection = connections
            .get(&tool.server)
            .ok_or_else(|| t!("mcp.not_running", server = tool.server).to_string())?;
        let mut params = CallToolRequestParams::new(tool.name.clone());
        if let Value::Object(arguments) = arguments {
            params = params.with_arguments(arguments);
        }
        let call = connection.client.call_tool(params);
        let result = tokio::time::timeout(CALL_TIMEOUT, call)
            .await
            .map_err(|_| t!("mcp.timeout").to_string())?
            .map_err(|e| t!("mcp.call_failed", error = e).to_string())?;

        let text = result_text(&result.content, result.structured_content.as_ref());
        if result.is_error == Some(true) {
            Err(text)
        } else {
            Ok(text)
        }
    }

    fn publish(
        &self,
        config: &McpConfig,
        connections: &BTreeMap<String, Connection>,
        failures: &HashMap<String, String>,
        starting: Vec<&str>,
    ) {
        let servers = config
            .mcp_servers
            .iter()
            .map(|(name, server)| {
                let connection = connections.get(name);
                let state = if server.disabled {
                    ServerState::Disabled
                } else if connection.is_some() {
                    ServerState::Running
                } else if starting.contains(&name.as_str()) {
                    ServerState::Starting
                } else {
                    ServerState::Failed
                };
                ServerStatus {
                    name: name.clone(),
                    state,
                    error: failures.get(name).cloned(),
                    tools: connection.map_or(0, |c| c.tools.len()),
                    logged_in: oauth::is_logged_in(name),
                }
            })
            .collect();
        let tools = connections.values().flat_map(|c| c.tools.iter().cloned()).collect();
        *self.catalog.write().expect("mcp catalog lock poisoned") = Catalog { tools, servers };
    }
}

async fn connect(name: &str, server: &ServerConfig, http: &reqwest::Client) -> Result<Connection, String> {
    let client = match server.transport()? {
        Transport::Process { command } => {
            let mut cmd = rmcp::transport::which_command(command)
                .map_err(|_| t!("mcp.command_not_found", command = command).to_string())?;
            cmd.args(&server.args).creation_flags(CREATE_NO_WINDOW);
            for (key, value) in &server.env {
                cmd.env(key, resolve_secrets(value)?);
            }
            if let Some(cwd) = &server.cwd {
                cmd.current_dir(cwd);
            }
            let (transport, stderr) = TokioChildProcess::builder(cmd)
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| t!("mcp.start_failed", error = e).to_string())?;
            // Servers log to stderr; keep it in Cephi's log instead of a console window.
            if let Some(stderr) = stderr {
                let server_name = name.to_string();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        log::debug!("mcp {server_name}: {line}");
                    }
                });
            }
            ().serve(transport)
                .await
                .map_err(|e| t!("mcp.start_failed", error = e).to_string())?
        }
        Transport::Http { url } => {
            let mut headers = HashMap::new();
            for (key, value) in &server.headers {
                let name = key
                    .parse()
                    .map_err(|_| t!("mcp.invalid_header", header = key).to_string())?;
                let value = resolve_secrets(value)?
                    .parse()
                    .map_err(|_| t!("mcp.invalid_header", header = key).to_string())?;
                headers.insert(name, value);
            }
            let mut config = StreamableHttpClientTransportConfig::with_uri(url);
            config.custom_headers = headers;
            // Servers with a browser login get a client that sends (and refreshes) the token.
            match oauth::client(http, name, url).await? {
                Some(authorized) => {
                    let transport = StreamableHttpClientTransport::with_client(authorized, config);
                    ().serve(transport)
                        .await
                        .map_err(|e| t!("mcp.start_failed", error = e).to_string())?
                }
                None => {
                    let transport = StreamableHttpClientTransport::from_config(config);
                    ().serve(transport)
                        .await
                        .map_err(|e| t!("mcp.start_failed", error = e).to_string())?
                }
            }
        }
    };

    let listed = client
        .list_all_tools()
        .await
        .map_err(|e| t!("mcp.list_failed", error = e).to_string())?;
    let tools = listed
        .into_iter()
        .map(|tool| {
            let original = tool.name.to_string();
            let description = tool.description.as_deref().unwrap_or_default().to_string();
            McpTool {
                exposed: exposed_name(name, &original),
                title: tool
                    .annotations
                    .as_ref()
                    .and_then(|a| a.title.clone())
                    .or(tool.title.clone())
                    .unwrap_or_else(|| original.clone()),
                // The model sees which server a tool belongs to.
                description: format!("[{name}] {description}"),
                input_schema: Value::Object((*tool.input_schema).clone()),
                auto_approved: server.is_auto_approved(&original),
                server: name.to_string(),
                name: original,
            }
        })
        .collect();

    Ok(Connection {
        config: server.clone(),
        client,
        tools,
    })
}

/// `<server>__<tool>` with characters every model API accepts, at most 64 long.
fn exposed_name(server: &str, tool: &str) -> String {
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    let mut name = format!("{}{SEPARATOR}{}", clean(server), clean(tool));
    name.truncate(MAX_TOOL_NAME);
    name
}

/// Text for the model: text blocks as they are, other content summarized.
fn result_text(content: &[ContentBlock], structured: Option<&Value>) -> String {
    let parts: Vec<String> = content
        .iter()
        .map(|block| match block {
            ContentBlock::Text(text) => text.text.clone(),
            ContentBlock::Image(_) => "[image]".into(),
            ContentBlock::Audio(_) => "[audio]".into(),
            ContentBlock::Resource(_) | ContentBlock::ResourceLink(_) => "[resource]".into(),
            _ => "[content]".into(),
        })
        .collect();
    if parts.is_empty() {
        return structured.map(Value::to_string).unwrap_or_default();
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposed_names_are_safe_and_short() {
        assert_eq!(exposed_name("github", "search_code"), "github__search_code");
        assert_eq!(exposed_name("my server", "do.thing"), "my_server__do_thing");
        assert_eq!(exposed_name("s", &"x".repeat(100)).len(), 64);
    }

    /// Starts `tests/fixtures/echo-mcp-server.js` (needs Node.js), lists its tool and calls it.
    #[test]
    fn talks_to_a_real_stdio_server() {
        if rmcp::transport::which_command("node").is_err() {
            eprintln!("skipped: node not installed");
            return;
        }
        // Most servers start with npx, which is npx.cmd on Windows.
        assert!(
            rmcp::transport::which_command("npx").is_ok(),
            "npx is found next to node"
        );
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/echo-mcp-server.js");
        let config: McpConfig = serde_json::from_value(serde_json::json!({
            "mcpServers": { "echo": { "command": "node", "args": [script], "autoApprove": ["echo"] } }
        }))
        .unwrap();

        tauri::async_runtime::block_on(async {
            let manager = McpManager::default();
            manager.sync(&config).await;
            let servers = manager.servers();
            assert_eq!(servers[0].state, ServerState::Running, "{:?}", servers[0].error);

            let tool = manager.find("echo__echo").expect("tool is listed");
            assert!(tool.auto_approved);
            assert!(tool.description.starts_with("[echo]"));

            let answer = manager.call(&tool, serde_json::json!({ "text": "hi" })).await.unwrap();
            assert_eq!(answer, "echo: hi");

            // Removing the server from the config stops it and drops its tools.
            manager.sync(&McpConfig::default()).await;
            assert!(manager.find("echo__echo").is_none());
        });
    }

    #[test]
    fn empty_config_means_no_servers() {
        let manager = McpManager::default();
        tauri::async_runtime::block_on(manager.sync(&McpConfig::default()));
        assert!(manager.tools().is_empty() && manager.servers().is_empty());
    }
}
