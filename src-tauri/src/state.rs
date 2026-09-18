use std::sync::{Arc, Mutex, RwLock};

use crate::agent::AgentController;
use crate::config::{Config, ConfigPaths};
use crate::desktop::web::WebController;
use crate::history::ChatStore;
use crate::mcp::McpManager;
use crate::desktop::window::WindowController;
use crate::integrations::spotify::SpotifyClient;
use crate::launcher::AppIndex;

/// Global application state, managed by Tauri.
///
/// Configuration is stored as an immutable snapshot (`Arc<Config>`) that is swapped
/// atomically on reload, so long-running tasks keep a consistent view without holding locks.
pub struct AppState {
    pub paths: ConfigPaths,
    pub http: reqwest::Client,
    pub agent: AgentController,
    pub window: WindowController,
    pub web: WebController,
    /// Connections to the MCP servers from `mcp.json`.
    pub mcp: McpManager,
    pub launcher: AppIndex,
    /// The current conversation on disk.
    pub chats: ChatStore,
    pub spotify: SpotifyClient,
    config: RwLock<Arc<Config>>,
    hotkey_error: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(paths: ConfigPaths, config: Config, http: reqwest::Client, launcher: AppIndex, chats: ChatStore) -> Self {
        Self {
            paths,
            http,
            agent: AgentController::default(),
            window: WindowController::default(),
            web: WebController::default(),
            mcp: McpManager::default(),
            launcher,
            chats,
            spotify: SpotifyClient::default(),
            config: RwLock::new(Arc::new(config)),
            hotkey_error: Mutex::new(None),
        }
    }

    pub fn config(&self) -> Arc<Config> {
        self.config.read().expect("config lock poisoned").clone()
    }

    pub fn replace_config(&self, config: Config) {
        *self.config.write().expect("config lock poisoned") = Arc::new(config);
    }

    pub fn hotkey_error(&self) -> Option<String> {
        self.hotkey_error.lock().expect("hotkey lock poisoned").clone()
    }

    pub fn set_hotkey_error(&self, error: Option<String>) {
        *self.hotkey_error.lock().expect("hotkey lock poisoned") = error;
    }
}
