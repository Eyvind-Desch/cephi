use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Contents of `settings.json`. Every field is optional in the file; missing values use defaults.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    #[serde(rename = "$schema", skip_serializing)]
    pub schema: Option<String>,
    pub hotkey: String,
    pub default_profile: String,
    pub profiles: Vec<Profile>,
    pub ollama: OllamaSettings,
    pub agent: AgentSettings,
    pub window: WindowSettings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub model: String,
    /// Whether the model may call tools.
    #[serde(default = "default_true")]
    pub tools: bool,
    /// Enable/disable the model's reasoning mode. `None` = model default.
    #[serde(default)]
    pub think: Option<bool>,
    #[serde(default)]
    pub temperature: Option<f32>,
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

    fn validate(&self) -> Result<(), String> {
        if self.profiles.is_empty() {
            return Err("Mindestens ein Profil in \"profiles\" ist nötig.".into());
        }
        for (i, profile) in self.profiles.iter().enumerate() {
            if profile.id.trim().is_empty() || profile.model.trim().is_empty() {
                return Err(format!("profiles[{i}]: \"id\" und \"model\" dürfen nicht leer sein."));
            }
            if self.profiles[..i].iter().any(|p| p.id == profile.id) {
                return Err(format!("profiles: doppelte id \"{}\".", profile.id));
            }
        }
        if !self.profiles.iter().any(|p| p.id == self.default_profile) {
            return Err(format!(
                "defaultProfile \"{}\" existiert nicht in \"profiles\".",
                self.default_profile
            ));
        }
        if url::Url::parse(&self.ollama.base_url).is_err() {
            return Err(format!(
                "ollama.baseUrl \"{}\" ist keine gültige URL.",
                self.ollama.base_url
            ));
        }
        if self.agent.max_tool_rounds == 0 {
            return Err("agent.maxToolRounds muss mindestens 1 sein.".into());
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Result<Settings, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
        Err(e) => return Err(format!("Datei nicht lesbar: {e}")),
    };
    let settings: Settings = serde_json::from_str(&text).map_err(|e| format!("Ungültiges JSON: {e}"))?;
    settings.validate()?;
    Ok(settings)
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: None,
            // Not Ctrl+Space: that is IntelliSense in VS Code and many IDEs.
            hotkey: "Ctrl+Alt+Space".into(),
            default_profile: "fast".into(),
            profiles: vec![
                Profile {
                    id: "fast".into(),
                    label: "Schnell".into(),
                    model: "assistant-fast:latest".into(),
                    tools: true,
                    think: None,
                    temperature: None,
                },
                Profile {
                    id: "smart".into(),
                    label: "Smart".into(),
                    model: "assistant:latest".into(),
                    tools: true,
                    think: Some(false),
                    temperature: None,
                },
                Profile {
                    id: "deep".into(),
                    label: "Gründlich".into(),
                    model: "assistant-big:latest".into(),
                    tools: true,
                    think: None,
                    temperature: None,
                },
            ],
            ollama: OllamaSettings::default(),
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
            system_prompt: "Du bist Orbit, ein hilfreicher Desktop-Assistent unter Windows. \
                Antworte knapp und in der Sprache des Nutzers. Nutze Tools, wenn sie helfen, \
                und erfinde keine Tool-Ergebnisse."
                .into(),
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
