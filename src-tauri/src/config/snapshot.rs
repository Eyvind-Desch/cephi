use serde::Serialize;

use super::{Config, Diagnostic};
use crate::tools::Permission;

/// Read-only view of the configuration sent to the frontend. Texts are already resolved for the
/// active language.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSnapshot {
    /// Active UI language (`settings.language` resolved, e.g. `system` → `de`).
    pub language: String,
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
    /// Provider id (`ollama` for local models).
    pub provider: String,
    /// Display name of the provider.
    pub provider_label: String,
    pub local: bool,
    /// False when the provider's API key is missing.
    pub available: bool,
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
        let locale = rust_i18n::locale();
        Self {
            language: locale.to_string(),
            config_dir,
            hotkey: settings.hotkey.clone(),
            default_profile: settings.default_profile.clone(),
            profiles: settings
                .profiles
                .iter()
                .map(|p| {
                    let provider = settings.provider(&p.provider);
                    ProfileInfo {
                        id: p.id.clone(),
                        label: p.label.resolve(&locale).to_string(),
                        model: p.model.clone(),
                        tools: p.tools,
                        provider: p.provider.clone(),
                        provider_label: provider.map_or_else(|| "Ollama".to_string(), |pr| pr.display_name()),
                        local: provider.is_none(),
                        available: provider.is_none_or(|pr| crate::secrets::is_set(&pr.api_key_secret())),
                    }
                })
                .collect(),
            tools: config
                .tools
                .iter()
                .map(|t| ToolInfo {
                    name: t.manifest.name.clone(),
                    title: t.title(),
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
