//! User configuration: `settings.json`, tool manifests in `tools/` and MCP servers in `mcp.json`.
//!
//! Everything lives in a single user-editable folder (see [`ConfigPaths`]). Loading never
//! fails hard: problems are collected as [`Diagnostic`]s and shown in the UI, and a broken
//! `settings.json` falls back to the last good settings.

pub(crate) mod defaults;
pub mod edit;
mod paths;
mod settings;
mod snapshot;
pub mod watcher;

use std::path::Path;

use serde::Serialize;

use crate::mcp::config::{self as mcp_config, McpConfig};
use crate::tools::ToolRegistry;

pub use paths::ConfigPaths;
pub use settings::{
    Anchor, OllamaSettings, Profile, ProviderKind, ProviderSettings, SearchSettings, Settings, Shortcut,
    ShortcutAction, TranslateSettings, VoiceSettings, WindowSettings,
};
pub use snapshot::ConfigSnapshot;

/// A problem found while loading configuration, attributed to a file.
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub source: String,
    pub message: String,
}

impl Diagnostic {
    pub fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
        }
    }

    pub fn for_path(root: &Path, path: &Path, message: impl Into<String>) -> Self {
        let source = path.strip_prefix(root).unwrap_or(path).display().to_string();
        Self::new(source.replace('\\', "/"), message)
    }
}

/// A fully loaded, immutable configuration snapshot.
pub struct Config {
    pub settings: Settings,
    pub tools: ToolRegistry,
    pub mcp: McpConfig,
    pub diagnostics: Vec<Diagnostic>,
}

/// Loads settings and tools from disk.
///
/// If `settings.json` is invalid, `previous` (or the built-in defaults) is used instead.
/// Also activates the configured language, so all following messages (including the tool
/// diagnostics collected here) are localized.
pub fn load(paths: &ConfigPaths, previous: Option<&Settings>) -> Config {
    let mut diagnostics = Vec::new();

    let settings = match settings::load(&paths.settings) {
        Ok(settings) => settings,
        Err(message) => {
            diagnostics.push(Diagnostic::for_path(&paths.root, &paths.settings, message));
            previous.cloned().unwrap_or_default()
        }
    };
    let locale = crate::i18n::apply(settings.language);
    log::info!("language: {locale}");

    let (tools, tool_diagnostics) = ToolRegistry::load_dir(&paths.root, &paths.tools);
    diagnostics.extend(tool_diagnostics);

    let mcp = mcp_config::load(&paths.mcp).unwrap_or_else(|message| {
        diagnostics.push(Diagnostic::for_path(&paths.root, &paths.mcp, message));
        McpConfig::default()
    });

    for diagnostic in &diagnostics {
        log::warn!("config: {}: {}", diagnostic.source, diagnostic.message);
    }
    log::info!(
        "config loaded: {} tool(s), {} profile(s)",
        tools.len(),
        settings.profiles.len()
    );

    Config {
        settings,
        tools,
        mcp,
        diagnostics,
    }
}
