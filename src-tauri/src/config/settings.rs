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
    /// Picks up the text selected in the foreground app (copies it, then restores the clipboard).
    /// Empty disables it.
    pub selection_hotkey: Option<String>,
    pub default_profile: String,
    pub profiles: Vec<Profile>,
    /// The local provider (id `ollama`), always available.
    pub ollama: OllamaSettings,
    /// Cloud providers. API keys live in the vault as `<id>_api_key` (see `apiKeySecret`).
    pub providers: Vec<ProviderSettings>,
    pub search: SearchSettings,
    pub translate: TranslateSettings,
    pub voice: VoiceSettings,
    pub agent: AgentSettings,
    pub window: WindowSettings,
    /// Shortcuts the user added, on top of [`Settings::hotkey`] and [`Settings::selection_hotkey`].
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
}

/// A key combination the user bound to something Cephi can do.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Shortcut {
    /// Like the main hotkey, e.g. "Ctrl+Alt+S".
    pub key: String,
    pub action: ShortcutAction,
    /// `prompt` only: what is sent. A phrase that is also a quick action (e.g. "pause") runs
    /// straight away, without the model — the same fast path the input field uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutAction {
    /// Show Cephi (without hiding it again, unlike the main hotkey).
    Show,
    /// Pick up the text selected in the foreground app, like [`Settings::selection_hotkey`].
    Selection,
    /// Freeze the screen and pick a region to ask about.
    Screenshot,
    /// Start dictating, or stop and insert what was said.
    Dictate,
    NewChat,
    /// Send [`Shortcut::text`].
    Prompt,
    Translate,
    /// Open the notes tab with a new note (quick capture).
    Notes,
    Web,
    History,
    Tools,
    Settings,
}

impl Shortcut {
    fn key(&self) -> &str {
        self.key.trim()
    }
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
    /// Whether the model can look at pictures (screenshots). Left out, Cephi decides by provider and
    /// model name ([`Profile::sees_images`]); set it explicitly when that guess is wrong.
    #[serde(default)]
    pub vision: Option<bool>,
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

    /// Logo name for the UI and the page where keys are created, derived from type and base URL.
    pub fn brand(&self) -> (&'static str, Option<&'static str>) {
        let base_url = self.base_url.as_deref().unwrap_or_default();
        if self.kind == ProviderKind::Anthropic {
            ("anthropic", Some("https://console.anthropic.com/settings/keys"))
        } else if base_url.contains("generativelanguage.googleapis.com") {
            ("gemini", Some("https://aistudio.google.com/apikey"))
        } else if base_url.contains("api.openai.com") {
            ("openai", Some("https://platform.openai.com/api-keys"))
        } else if base_url.contains("openrouter.ai") {
            ("openrouter", Some("https://openrouter.ai/settings/keys"))
        } else if base_url.contains("groq.com") {
            ("ai", Some("https://console.groq.com/keys"))
        } else {
            ("ai", None)
        }
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
pub struct SearchSettings {
    pub searxng_url: String,
    /// SearXNG language: `auto`, `all` or a code such as `de`.
    pub language: String,
    pub max_results: usize,
    /// Let the AI answer from the results after `? query`. Off: only the result cards
    /// (a button asks the AI on demand).
    pub summarize: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct TranslateSettings {
    pub libretranslate_url: String,
    /// Text in this language is translated into `foreign_language`, everything else into this one.
    pub native_language: String,
    pub foreign_language: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct VoiceSettings {
    /// Whisper service (`infra/compose.yaml`); another host works too.
    pub whisper_url: String,
    /// Language of the dictation: a code such as `de` or `auto` (detect).
    pub language: String,
    /// Send the recognized text right away instead of putting it in the input field.
    pub send_directly: bool,
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
    /// Enlarged panel, e.g. for the web tab (limited to the screen).
    pub large: Size,
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

/// Parts of a local model's name that mean it can look at pictures.
const VISION_MODEL_HINTS: [&str; 9] = [
    "llava",
    "vision",
    "-vl",
    "vl:",
    "gemma3",
    "gemma4",
    "minicpm-v",
    "moondream",
    "pixtral",
];

impl Profile {
    /// Whether a screenshot may go to this model. Without `vision` in `settings.json`: cloud models
    /// can (all current ones read pictures), a local one only when its name says so — otherwise
    /// every profile of an older settings file would look blind.
    pub fn sees_images(&self) -> bool {
        if let Some(vision) = self.vision {
            return vision;
        }
        if self.provider != crate::llm::LOCAL_PROVIDER_ID {
            return true;
        }
        let model = self.model.to_lowercase();
        VISION_MODEL_HINTS.iter().any(|hint| model.contains(hint))
    }
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

    pub(crate) fn validate(&self) -> Result<(), String> {
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
        if url::Url::parse(&self.search.searxng_url).is_err() {
            return Err(t!("settings.invalid_searxng_url", url = self.search.searxng_url).into());
        }
        if url::Url::parse(&self.translate.libretranslate_url).is_err() {
            return Err(t!(
                "settings.invalid_libretranslate_url",
                url = self.translate.libretranslate_url
            )
            .into());
        }
        if !(1..=20).contains(&self.search.max_results) {
            return Err(t!("settings.max_results").into());
        }
        // A shortcut that silently does nothing is worse than a message on start.
        let taken = [Some(self.hotkey.trim()), self.selection_hotkey.as_deref().map(str::trim)];
        for (index, shortcut) in self.shortcuts.iter().enumerate() {
            if shortcut.key().is_empty() {
                return Err(t!("settings.shortcut_without_key", index = index).into());
            }
            let earlier = self.shortcuts[..index].iter().map(Shortcut::key);
            if taken.into_iter().flatten().chain(earlier).any(|key| key == shortcut.key()) {
                return Err(t!("settings.duplicate_shortcut", key = shortcut.key()).into());
            }
            if shortcut.action == ShortcutAction::Prompt
                && shortcut.text.as_deref().is_none_or(|text| text.trim().is_empty())
            {
                return Err(t!("settings.shortcut_without_text", key = shortcut.key()).into());
            }
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
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are Cephi, a desktop assistant on Windows. \
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
        vision: None,
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
            selection_hotkey: Some("Ctrl+Alt+T".into()),
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
            search: SearchSettings::default(),
            translate: TranslateSettings::default(),
            voice: VoiceSettings::default(),
            agent: AgentSettings::default(),
            window: WindowSettings::default(),
            shortcuts: Vec::new(),
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

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            // Matches the port in infra/searxng/compose.yaml.
            searxng_url: "http://127.0.0.1:8888".into(),
            language: "auto".into(),
            max_results: 6,
            summarize: true,
        }
    }
}

impl Default for TranslateSettings {
    fn default() -> Self {
        Self {
            // Matches the port in infra/compose.yaml.
            libretranslate_url: "http://127.0.0.1:8890".into(),
            native_language: "de".into(),
            foreign_language: "en".into(),
        }
    }
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            // Matches the port in infra/compose.yaml.
            whisper_url: "http://127.0.0.1:8891".into(),
            language: "auto".into(),
            send_directly: false,
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
            large: Size {
                width: 1040,
                height: 820,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(provider: &str, model: &str, vision: Option<bool>) -> Profile {
        Profile {
            id: "x".into(),
            label: LocalizedText::plain("X"),
            provider: provider.into(),
            model: model.into(),
            tools: true,
            vision,
            think: None,
            temperature: None,
            effort: None,
        }
    }

    /// An older settings.json has no "vision" anywhere; without a guess every model would look blind.
    #[test]
    fn models_that_can_see_are_recognised_without_a_setting() {
        assert!(profile("anthropic", "claude-opus-5", None).sees_images(), "cloud models read pictures");
        assert!(profile("gemini", "gemini-3.8-flash", None).sees_images());
        assert!(!profile("ollama", "assistant-fast:latest", None).sees_images(), "a plain local model does not");
        assert!(profile("ollama", "qwen2.5vl:7b", None).sees_images(), "local vision models do");
        assert!(profile("ollama", "llava:13b", None).sees_images());
        assert!(profile("ollama", "gemma3:12b", None).sees_images());
        // The setting always wins over the guess.
        assert!(!profile("anthropic", "claude-opus-5", Some(false)).sees_images());
        assert!(profile("ollama", "my-own-model", Some(true)).sees_images());
    }

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

    fn with_shortcuts(json: &str) -> Result<(), String> {
        serde_json::from_str::<Settings>(&format!(r#"{{ "shortcuts": {json} }}"#))
            .expect("valid JSON")
            .validate()
    }

    #[test]
    fn a_shortcut_that_could_not_work_is_reported() {
        with_shortcuts(r#"[{ "key": "Ctrl+Alt+S", "action": "screenshot" }]"#).unwrap();
        with_shortcuts(r#"[{ "key": "Ctrl+Alt+P", "action": "prompt", "text": "pause" }]"#).unwrap();

        assert!(with_shortcuts(r#"[{ "key": " ", "action": "show" }]"#).is_err(), "no key");
        assert!(
            with_shortcuts(r#"[{ "key": "Ctrl+Alt+P", "action": "prompt" }]"#).is_err(),
            "a message without a text"
        );
        assert!(
            with_shortcuts(r#"[{ "key": "Ctrl+Alt+P", "action": "prompt", "text": "  " }]"#).is_err(),
            "a message that is only spaces"
        );
        assert!(
            with_shortcuts(r#"[{ "key": "Ctrl+Alt+S", "action": "show" }, { "key": "Ctrl+Alt+S", "action": "web" }]"#)
                .is_err(),
            "the same combination twice"
        );
        // The two built-in hotkeys are taken as well; only one of them can win.
        assert!(
            with_shortcuts(r#"[{ "key": "Ctrl+Alt+Space", "action": "web" }]"#).is_err(),
            "the main hotkey"
        );
        assert!(
            with_shortcuts(r#"[{ "key": "Ctrl+Alt+T", "action": "web" }]"#).is_err(),
            "the selection hotkey"
        );
    }
}
