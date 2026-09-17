use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::args::{self, Args};
use super::manifest::ToolManifest;
use super::trigger::Trigger;
use crate::config::Diagnostic;

/// A loaded, validated and enabled tool.
pub struct Tool {
    pub manifest: ToolManifest,
    /// Path relative to the config root, for display.
    pub source: String,
}

impl Tool {
    pub fn title(&self) -> &str {
        self.manifest.title.as_deref().unwrap_or(&self.manifest.name)
    }

    pub fn normalize_args(&self, raw: &Value) -> Result<Args, String> {
        args::normalize(&self.manifest.parameters, raw)
    }

    /// Tool definition in the format expected by Ollama's `/api/chat`.
    fn definition(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.manifest.name,
                "description": self.manifest.description,
                "parameters": self.manifest.parameters,
            }
        })
    }
}

/// All enabled tools, keyed by name, plus their compiled fast-path triggers.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Tool>,
    /// `(tool name, trigger)`, most specific first.
    triggers: Vec<(String, Trigger)>,
}

impl ToolRegistry {
    /// Loads every `*.json` below `dir` (recursively). Invalid files are skipped and reported.
    pub fn load_dir(root: &Path, dir: &Path) -> (Self, Vec<Diagnostic>) {
        let mut registry = Self::default();
        let mut diagnostics = Vec::new();

        let mut files = Vec::new();
        if let Err(e) = collect_json_files(dir, &mut files) {
            if dir.exists() {
                diagnostics.push(Diagnostic::for_path(root, dir, format!("Ordner nicht lesbar: {e}")));
            }
        }
        files.sort();

        for path in files {
            let loaded = load_file(&path).and_then(|manifest| {
                let triggers = manifest.compile_triggers()?;
                Ok((manifest, triggers))
            });
            match loaded {
                Ok((manifest, _)) if !manifest.enabled => {}
                Ok((manifest, triggers)) => {
                    let source = Diagnostic::for_path(root, &path, "").source;
                    if let Some(existing) = registry.tools.get(&manifest.name) {
                        diagnostics.push(Diagnostic::for_path(
                            root,
                            &path,
                            format!(
                                "Tool-Name \"{}\" ist bereits in {} vergeben.",
                                manifest.name, existing.source
                            ),
                        ));
                        continue;
                    }
                    let name = manifest.name.clone();
                    registry
                        .triggers
                        .extend(triggers.into_iter().map(|t| (name.clone(), t)));
                    registry.tools.insert(name, Tool { manifest, source });
                }
                Err(message) => diagnostics.push(Diagnostic::for_path(root, &path, message)),
            }
        }

        // Stable sort: equally specific triggers keep file order.
        registry
            .triggers
            .sort_by_key(|(_, trigger)| std::cmp::Reverse(trigger.specificity));
        (registry, diagnostics)
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Tool> {
        self.tools.values()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools.values().map(Tool::definition).collect()
    }

    /// All trigger matches for `input`, most specific first, with raw (unvalidated) arguments.
    pub fn trigger_matches<'a>(&'a self, input: &'a str) -> impl Iterator<Item = (&'a Tool, Value)> + 'a {
        self.triggers
            .iter()
            .filter_map(move |(name, trigger)| Some((self.tools.get(name)?, trigger.matches(input)?)))
    }

    /// Secret name → titles of the tools that use it.
    pub fn secret_usage(&self) -> BTreeMap<String, Vec<String>> {
        let mut usage: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for tool in self.tools.values() {
            for secret in tool.manifest.secrets() {
                usage.entry(secret).or_default().push(tool.title().to_string());
            }
        }
        usage
    }
}

fn load_file(path: &Path) -> Result<ToolManifest, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("Datei nicht lesbar: {e}"))?;
    let manifest: ToolManifest = serde_json::from_str(&text).map_err(|e| format!("Ungültiges Tool: {e}"))?;
    manifest.validate()?;
    Ok(manifest)
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("json")) {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loads the shipped default tools into a temporary folder.
    fn default_registry() -> ToolRegistry {
        let dir = std::env::temp_dir().join(format!("orbit-registry-test-{}", uuid::Uuid::new_v4()));
        for (relative, content) in crate::config::defaults::TOOLS {
            let path = dir.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        let (registry, diagnostics) = ToolRegistry::load_dir(&dir, &dir);
        let _ = fs::remove_dir_all(&dir);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        registry
    }

    fn first_match(registry: &ToolRegistry, input: &str) -> Option<(String, Value)> {
        registry
            .trigger_matches(input)
            .next()
            .map(|(tool, args)| (tool.manifest.name.clone(), args))
    }

    #[test]
    fn default_triggers_route_common_phrases() {
        let registry = default_registry();
        let cases = [
            ("Spiele Another Love von Tom Odell in Spotify", "spotify_play"),
            ("öffne Another Love auf Spotify", "spotify_play"),
            ("spiele das album Divide auf spotify", "spotify_play"),
            ("öffne Discord", "open_app"),
            ("starte die app Steam", "open_app"),
            ("Öffne den Rechner", "open_app"),
            ("g rust tauri tutorial", "google_search"),
            ("yt lofi hip hop", "youtube_search"),
            ("nächster Song", "media_control"),
            ("pause", "media_control"),
            ("lauter", "media_control"),
            ("mach es leiser", "media_control"),
            ("schalte bluetooth jetzt an", "radio"),
            ("WLAN aus", "radio"),
            ("ist bluetooth an?", "radio"),
        ];
        for (input, expected) in cases {
            let (tool, _) = first_match(&registry, input).unwrap_or_else(|| panic!("no match for {input:?}"));
            assert_eq!(tool, expected, "{input:?}");
        }
    }

    #[test]
    fn spotify_trigger_beats_generic_open_app() {
        let registry = default_registry();
        let (tool, args) = first_match(&registry, "öffne another love in spotify").unwrap();
        assert_eq!(tool, "spotify_play");
        assert_eq!(args["query"], "another love");
    }

    /// First trigger whose arguments pass validation, like the agent's fast path.
    fn first_valid(registry: &ToolRegistry, input: &str) -> Option<(String, Args)> {
        registry
            .trigger_matches(input)
            .find_map(|(tool, raw)| Some((tool.manifest.name.clone(), tool.normalize_args(&raw).ok()?)))
    }

    #[test]
    fn volume_phrases() {
        let registry = default_registry();
        let cases = [
            ("mach die Lautstärke auf 30", "set_volume", Some(("level", 30))),
            ("Lautstärke 30%", "set_volume", Some(("level", 30))),
            ("stell die lautstärke auf 45 prozent", "set_volume", Some(("level", 45))),
            ("mach 24 lauter", "volume_up", Some(("amount", 24))),
            ("mach es 10 % leiser", "volume_down", Some(("amount", 10))),
            ("mach es lauter", "volume_up", None),
            ("ton an", "unmute", None),
        ];
        for (input, action, number) in cases {
            let (tool, args) = first_valid(&registry, input).unwrap_or_else(|| panic!("no valid match for {input:?}"));
            assert_eq!(tool, "media_control", "{input:?}");
            assert_eq!(args["action"], action, "{input:?}");
            if let Some((key, value)) = number {
                assert_eq!(args[key], value, "{input:?}");
            }
        }
    }

    #[test]
    fn radio_trigger_only_accepts_known_devices() {
        let registry = default_registry();
        let (tool, args) = first_valid(&registry, "Schalte Bluetooth jetzt an").unwrap();
        assert_eq!(
            (tool.as_str(), args["device"].as_str(), args["state"].as_str()),
            ("radio", Some("Bluetooth"), Some("on"))
        );
        // Matches the pattern, but "licht" is no radio: the builtin rejects the fast path (checked in builtin.rs).
        assert!(crate::integrations::radio::Device::parse("das licht").is_err());
        // "ton an" belongs to media control, which is more specific.
        assert_eq!(first_valid(&registry, "ton an").unwrap().0, "media_control");
    }

    #[test]
    fn articles_are_not_part_of_the_app_name() {
        let registry = default_registry();
        let (_, args) = first_match(&registry, "Öffne den Rechner").unwrap();
        assert_eq!(args["name"], "Rechner");
    }

    #[test]
    fn questions_do_not_trigger() {
        let registry = default_registry();
        for input in [
            "Was ist die Hauptstadt von Frankreich?",
            "Wie ist das Wetter in Berlin?",
            "pausen im job",
        ] {
            assert!(
                first_match(&registry, input).is_none(),
                "{input:?} should go to the model"
            );
        }
    }
}
