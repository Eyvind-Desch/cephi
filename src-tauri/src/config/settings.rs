use std::fs;
use std::path::Path;

use rust_i18n::t;
use serde::{Deserialize, Serialize};

use crate::i18n::LanguageSetting;
use crate::tools::LocalizedText;

/// Contents of `settings.json`. Every field is optional in the file; missing values use defaults.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    #[serde(rename = "$schema", skip_serializing)]
    pub schema: Option<String>,
    pub language: LanguageSetting,
    pub hotkey: String,
    pub default_profile: String,
    pub profiles: Vec<Profile>,
    /// The local provider (id `ollama`), always available.
    pub ollama: OllamaSettings,
    /// Cloud providers. API keys live in the vault as `<id>_api_key` (see `apiKeySecret`).
    pub providers: Vec<ProviderSettings>,
    pub agent: AgentSettings,
    pub window: WindowSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub label: LocalizedText,
    /// Provider id: `ollama` (default) or an id from `providers`.
    #[serde(default = "default_provider")]
    pub provider: String,
    pub model: String,
    /// Whether the model may call tools.
    #[serde(default = "default_true")]
    pub tools: bool,
    /// Ollama: enable/disable the model's reasoning mode. `None` = model default.
    #[serde(default)]
    pub think: Option<bool>,
    /// Ollama and OpenAI-compatible providers. Not sent to Anthropic (current Claude models reject it).
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Anthropic: `low` | `medium` | `high` | `xhigh` | `max`. Lower is faster.
    #[serde(default)]
    pub effort: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    /// Anthropic Messages API (Claude).
    Anthropic,
    /// OpenAI Chat Completions or a compatible API (Gemini, OpenRouter, Groq, LM Studio, …).
    Openai,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderSettings {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: ProviderKind,
    #[serde(default)]
    pub label: Option<LocalizedText>,
    /// Required for `openai`; optional for `anthropic`.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Vault key name; default `<id>_api_key`.
    #[serde(default)]
    pub api_key_secret: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Anthropic: re-run declined requests on the recommended fallback model (server-side).
    #[serde(default = "default_true")]
    pub fallbacks: bool,
}

impl ProviderSettings {
    pub fn api_key_secret(&self) -> String {
        self.api_key_secret
            .clone()
            .unwrap_or_else(|| format!("{}_api_key", self.id))
    }

    pub fn display_name(&self) -> String {
        match &self.label {
            Some(label) => label.resolve(&rust_i18n::locale()).to_string(),
            None => self.id.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct OllamaSettings {
    pub base_url: String,
    /// How long Ollama keeps the model loaded after a request, e.g. `"10m"`.
    pub keep_alive: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentSettings {
    pub system_prompt: String,
    pub max_tool_rounds: u32,
    pub max_tool_output_chars: usize,
    pub confirm_timeout_secs: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowSettings {
    pub anchor: Anchor,
    pub margin: u32,
    pub hide_on_blur: bool,
    pub show_on_startup: bool,
    pub compact: Size,
    pub expanded: Size,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    BottomRight,
    BottomCenter,
    BottomLeft,
    TopRight,
    TopCenter,
    TopLeft,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Settings {
    /// Returns the requested profile, falling back to the default and then the first profile.
    pub fn profile(&self, id: Option<&str>) -> &Profile {
        id.and_then(|id| self.profiles.iter().find(|p| p.id == id))
            .or_else(|| self.profiles.iter().find(|p| p.id == self.default_profile))
            .unwrap_or(&self.profiles[0])
    }

    pub fn provider(&self, id: &str) -> Option<&ProviderSettings> {
        self.providers.iter().find(|p| p.id == id)
    }

    fn validate(&self) -> Result<(), String> {
        for (index, provider) in self.providers.iter().enumerate() {
            if provider.id == crate::llm::LOCAL_PROVIDER_ID || !crate::secrets::is_valid_name(&provider.id) {
                return Err(t!("settings.invalid_provider_id", id = provider.id).into());
            }
            if self.providers[..index].iter().any(|p| p.id == provider.id) {
                return Err(t!("settings.duplicate_provider", id = provider.id).into());
            }
            match (&provider.base_url, provider.kind) {
                (Some(url), _) if url::Url::parse(url).is_err() => {
                    return Err(t!("settings.invalid_provider_url", id = provider.id, url = url).into());
                }
                (None, ProviderKind::Openai) => {
                    return Err(t!("settings.provider_needs_url", id = provider.id).into());
                }
                _ => {}
            }
            if !crate::secrets::is_valid_name(&provider.api_key_secret()) {
                return Err(t!("secrets.invalid_name", name = provider.api_key_secret()).into());
            }
        }

        if self.profiles.is_empty() {
            return Err(t!("settings.no_profiles").into());
        }
        for (index, profile) in self.profiles.iter().enumerate() {
            if profile.id.trim().is_empty() || profile.model.trim().is_empty() {
                return Err(t!("settings.profile_incomplete", index = index).into());
            }
            if self.profiles[..index].iter().any(|p| p.id == profile.id) {
                return Err(t!("settings.duplicate_profile", id = profile.id).into());
            }
            if profile.provider != crate::llm::LOCAL_PROVIDER_ID && self.provider(&profile.provider).is_none() {
                let (id, provider) = (&profile.id, &profile.provider);
                return Err(t!("settings.unknown_provider", profile = id, provider = provider).into());
            }
        }
        if !self.profiles.iter().any(|p| p.id == self.default_profile) {
            return Err(t!("settings.unknown_default_profile", id = self.default_profile).into());
        }
        if url::Url::parse(&self.ollama.base_url).is_err() {
            return Err(t!("settings.invalid_ollama_url", url = self.ollama.base_url).into());
        }
        if self.agent.max_tool_rounds == 0 {
            return Err(t!("settings.max_tool_rounds").into());
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Result<Settings, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(e) => return Err(t!("config.file_unreadable", error = e).into()),
    };
    let settings: Settings =
        serde_json::from_str(&text).map_err(|e| t!("config.invalid_json", error = e).to_string())?;
    settings.validate()?;
    Ok(settings)
}

/// Must match `agent.systemPrompt` in `assets/defaults/settings.json` (checked by a test).
///
/// Small models tend to invent device states ("Bluetooth is already on") and then insist on
/// them. The prompt makes tool results the only source of truth about the PC. Written in English
/// (models follow English instructions most reliably) and asks for answers in the user's language.
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are Orbit, a desktop assistant on Windows. \
Answer briefly and in the user's language.\n\
Rules:\n\
- You only know about the PC what tools report. Never claim to have checked, switched on or done \
something unless a tool result in this conversation proves it.\n\
- If no tool fits, say honestly that you cannot do it and suggest how the user can do it themselves.\n\
- If the user disagrees, check the state again with a tool instead of insisting on your statement.\n\
- Never invent tool results.";

fn default_true() -> bool {
    true
}

fn default_provider() -> String {
    crate::llm::LOCAL_PROVIDER_ID.into()
}

fn local_profile(id: &str, label: LocalizedText, model: &str) -> Profile {
    Profile {
        id: id.into(),
        label,
        provider: default_provider(),
        model: model.into(),
        tools: true,
        think: None,
        temperature: None,
        effort: None,
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: None,
            language: LanguageSetting::System,
            // Not Ctrl+Space: that is IntelliSense in VS Code and many IDEs.
            hotkey: "Ctrl+Alt+Space".into(),
            default_profile: "fast".into(),
            profiles: vec![
                Profile {
                    think: None,
                    ..local_profile(
                        "fast",
                        LocalizedText::translated(&[("en", "Fast"), ("de", "Schnell")]),
                        "assistant-fast:latest",
                    )
                },
                Profile {
                    think: Some(false),
                    ..local_profile("smart", LocalizedText::plain("Smart"), "assistant:latest")
                },
                local_profile(
                    "deep",
                    LocalizedText::translated(&[("en", "Thorough"), ("de", "Gründlich")]),
                    "assistant-big:latest",
                ),
                Profile {
                    provider: "anthropic".into(),
                    effort: Some("low".into()),
                    ..local_profile("claude", LocalizedText::plain("Claude"), "claude-opus-5")
                },
                Profile {
                    provider: "gemini".into(),
                    ..local_profile("gemini", LocalizedText::plain("Gemini"), "gemini-3.8-flash")
                },
            ],
            ollama: OllamaSettings::default(),
            providers: vec![
                ProviderSettings {
                    id: "anthropic".into(),
                    kind: ProviderKind::Anthropic,
                    label: Some(LocalizedText::plain("Anthropic")),
                    base_url: None,
                    api_key_secret: None,
                    max_tokens: None,
                    fallbacks: true,
                },
                ProviderSettings {
                    id: "gemini".into(),
                    kind: ProviderKind::Openai,
                    label: Some(LocalizedText::plain("Google Gemini")),
                    base_url: Some("https://generativelanguage.googleapis.com/v1beta/openai".into()),
                    api_key_secret: None,
                    max_tokens: None,
                    fallbacks: true,
                },
                ProviderSettings {
                    id: "openai".into(),
                    kind: ProviderKind::Openai,
                    label: Some(LocalizedText::plain("OpenAI")),
                    base_url: Some("https://api.openai.com/v1".into()),
                    api_key_secret: None,
                    max_tokens: None,
                    fallbacks: true,
                },
            ],
            agent: AgentSettings::default(),
            window: WindowSettings::default(),
        }
    }
}

impl Default for OllamaSettings {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".into(),
            keep_alive: "10m".into(),
        }
    }
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            system_prompt: DEFAULT_SYSTEM_PROMPT.into(),
            max_tool_rounds: 6,
            max_tool_output_chars: 8000,
            confirm_timeout_secs: 120,
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            anchor: Anchor::BottomRight,
            margin: 16,
            hide_on_blur: true,
            show_on_startup: true,
            compact: Size { width: 640, height: 68 },
            expanded: Size {
                width: 480,
                height: 720,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_file_is_valid() {
        let settings: Settings = serde_json::from_str(super::super::defaults::SETTINGS).unwrap();
        settings.validate().unwrap();
        assert_eq!(
            settings.agent.system_prompt, DEFAULT_SYSTEM_PROMPT,
            "defaults/settings.json is out of sync"
        );

        // The Rust defaults (used when settings.json is missing) must describe the same setup.
        let fallback = Settings::default();
        fallback.validate().unwrap();
        let profiles = |s: &Settings| {
            s.profiles
                .iter()
                .map(|p| (p.id.clone(), p.provider.clone(), p.model.clone()))
                .collect::<Vec<_>>()
        };
        let providers = |s: &Settings| {
            s.providers
                .iter()
                .map(|p| (p.id.clone(), p.kind, p.base_url.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(profiles(&settings), profiles(&fallback));
        assert_eq!(providers(&settings), providers(&fallback));
    }

    #[test]
    fn rejects_profiles_with_unknown_providers() {
        let settings: Settings = serde_json::from_str(
            r#"{ "defaultProfile": "x", "profiles": [{ "id": "x", "label": "X", "provider": "nope", "model": "m" }] }"#,
        )
        .unwrap();
        assert!(settings.validate().unwrap_err().contains("nope"));
    }

    #[test]
    fn openai_providers_need_a_base_url() {
        let settings: Settings =
            serde_json::from_str(r#"{ "providers": [{ "id": "groq", "type": "openai" }] }"#).unwrap();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn partial_file_uses_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "hotkey": "Alt+O" }"#).unwrap();
        assert_eq!(settings.hotkey, "Alt+O");
        assert_eq!(settings.window.margin, 16);
        settings.validate().unwrap();
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(serde_json::from_str::<Settings>(r#"{ "hotkeyy": "Alt+O" }"#).is_err());
    }
}
