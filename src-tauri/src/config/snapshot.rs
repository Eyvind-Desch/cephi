use serde::Serialize;

use super::{Config, Diagnostic};
use crate::tools::Permission;

/// Read-only view of the configuration sent to the frontend.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSnapshot {
    pub config_dir: String,
    pub hotkey: String,
    pub default_profile: String,
    pub profiles: Vec<ProfileInfo>,
    pub tools: Vec<ToolInfo>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInfo {
    pub id: String,
    pub label: String,
    pub model: String,
    pub tools: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    pub title: String,
    pub description: String,
    pub permission: Permission,
    pub kind: &'static str,
    pub source: String,
}

impl ConfigSnapshot {
    pub fn build(config: &Config, config_dir: String, extra: Option<Diagnostic>) -> Self {
        let settings = &config.settings;
        Self {
            config_dir,
            hotkey: settings.hotkey.clone(),
            default_profile: settings.default_profile.clone(),
            profiles: settings
                .profiles
                .iter()
                .map(|p| ProfileInfo {
                    id: p.id.clone(),
                    label: p.label.clone(),
                    model: p.model.clone(),
                    tools: p.tools,
                })
                .collect(),
            tools: config
                .tools
                .iter()
                .map(|t| ToolInfo {
                    name: t.manifest.name.clone(),
                    title: t.title().to_string(),
                    description: t.manifest.description.clone(),
                    permission: t.manifest.permission,
                    kind: t.manifest.action.kind(),
                    source: t.source.clone(),
                })
                .collect(),
            diagnostics: config.diagnostics.iter().cloned().chain(extra).collect(),
        }
    }
}
