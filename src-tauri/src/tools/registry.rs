use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rust_i18n::t;
use serde_json::Value;

use super::args::{self, Args};
use super::builtin::Builtin;
use super::manifest::{Action, AfterTrigger, ToolManifest};
use super::trigger::Trigger;
use crate::config::{Diagnostic, Settings};
use crate::llm::ToolDef;

/// A loaded, validated and enabled tool.
pub struct Tool {
    pub manifest: ToolManifest,
    /// Path relative to the config root, for display.
    pub source: String,
}

impl Tool {
    /// Whether the model should answer from the result after a trigger ran this tool.
    /// Search results are only summarized when `search.summarize` is on.
    pub fn answers_after_trigger(&self, settings: &Settings) -> bool {
        match (&self.manifest.after_trigger, &self.manifest.action) {
            (AfterTrigger::Show, _) => false,
            (AfterTrigger::Answer, Action::Builtin { id: Builtin::WebSearch }) => settings.search.summarize,
            (AfterTrigger::Answer, _) => true,
        }
    }

    /// Display title in the active language; falls back to the technical name.
    pub fn title(&self) -> String {
        match &self.manifest.title {
            Some(title) => title.resolve(&rust_i18n::locale()).to_string(),
            None => self.manifest.name.clone(),
        }
    }

    /// What a request is matched against when not every tool fits into the budget: the title and
    /// keywords in the user's language, plus the trigger phrases, which are written the way people
    /// actually ask ("spiele … auf spotify").
    fn match_words(&self) -> String {
        let locale = rust_i18n::locale();
        let mut words = self.title();
        if let Some(keywords) = &self.manifest.keywords {
            words.push(' ');
            words.push_str(keywords.resolve(&locale));
        }
        for trigger in &self.manifest.triggers {
            words.push(' ');
            words.push_str(trigger.pattern());
        }
        words
    }

    pub fn normalize_args(&self, raw: &Value) -> Result<Args, String> {
        args::normalize(&self.manifest.parameters, raw)
    }

    /// Provider-neutral definition offered to the model.
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: self.manifest.name.clone(),
            description: self.manifest.description.clone(),
            parameters: serde_json::to_value(&self.manifest.parameters).unwrap_or_default(),
        }
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
                diagnostics.push(Diagnostic::for_path(
                    root,
                    dir,
                    t!("config.folder_unreadable", error = e),
                ));
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
                            t!("tools.duplicate_name", name = manifest.name, source = existing.source),
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

    /// What the model may call, each with its title in the user's language: the title is what a
    /// request usually names ("Websuche"), so `tools::select` matches on it.
    pub fn offered(&self) -> Vec<(ToolDef, String)> {
        self.tools
            .values()
            .map(|tool| (tool.definition(), tool.match_words()))
            .collect()
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
                usage.entry(secret).or_default().push(tool.title());
            }
        }
        usage
    }
}

fn load_file(path: &Path) -> Result<ToolManifest, String> {
    let text = fs::read_to_string(path).map_err(|e| t!("config.file_unreadable", error = e).to_string())?;
    let manifest: ToolManifest =
        serde_json::from_str(&text).map_err(|e| t!("tools.invalid_manifest", error = e).to_string())?;
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

    #[test]
    fn search_answers_follow_the_summarize_setting() {
        let registry = default_registry();
        let (search, open_app) = (registry.get("web_search").unwrap(), registry.get("open_app").unwrap());
        let mut settings = Settings::default();
        assert!(search.answers_after_trigger(&settings));
        assert!(!open_app.answers_after_trigger(&settings));

        settings.search.summarize = false;
        assert!(!search.answers_after_trigger(&settings));
        assert_eq!(
            registry
                .trigger_matches("? tauri")
                .next()
                .map(|(t, _)| t.manifest.name.as_str()),
            Some("web_search")
        );
    }

    /// Loads the shipped default tools into a temporary folder.
    fn default_registry() -> ToolRegistry {
        let dir = std::env::temp_dir().join(format!("cephi-registry-test-{}", uuid::Uuid::new_v4()));
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
            // English phrases
            ("play Mr. Brightside by The Killers on Spotify", "spotify_play"),
            ("open Discord", "open_app"),
            ("launch the calculator", "open_app"),
            ("next track", "media_control"),
            ("turn it up by 20", "media_control"),
            ("set the volume to 30", "media_control"),
            ("turn on bluetooth", "radio"),
            ("is wifi on?", "radio"),
            ("search lofi beats on youtube", "youtube_search"),
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
        assert!(crate::integrations::radio::Device::parse("the light").is_err());
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
