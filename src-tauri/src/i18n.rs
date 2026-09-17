//! Localization of user-facing messages.
//!
//! English is the source language. Translations live in `src-tauri/locales/{en,de}.json` and are
//! used via `rust_i18n::t!("key", var = value)`. The frontend has its own catalog
//! (`src/i18n/locales`). Rule of thumb: text a user can see goes through `t!`; log lines and
//! internal errors stay plain English.

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

/// `language` in `settings.json`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LanguageSetting {
    /// Follow the Windows display language.
    #[default]
    System,
    En,
    De,
}

/// Resolves the setting to a supported locale code.
pub fn resolve(setting: LanguageSetting) -> &'static str {
    match setting {
        LanguageSetting::En => "en",
        LanguageSetting::De => "de",
        LanguageSetting::System => system_language(),
    }
}

/// Activates the language for all following `t!` calls. Returns the locale code.
pub fn apply(setting: LanguageSetting) -> &'static str {
    let locale = resolve(setting);
    rust_i18n::set_locale(locale);
    locale
}

fn system_language() -> &'static str {
    const LANG_GERMAN: u16 = 0x07;
    const PRIMARY_LANGUAGE_MASK: u16 = 0x3ff;
    // SAFETY: no arguments, no preconditions.
    let lang_id = unsafe { GetUserDefaultUILanguage() };
    if lang_id & PRIMARY_LANGUAGE_MASK == LANG_GERMAN {
        "de"
    } else {
        "en"
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use serde_json::Value;

    const EN: &str = include_str!("../locales/en.json");
    const DE: &str = include_str!("../locales/de.json");

    fn flatten(prefix: &str, value: &Value, out: &mut BTreeMap<String, String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    if key.starts_with('_') {
                        continue;
                    }
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    flatten(&path, child, out);
                }
            }
            Value::String(text) => {
                out.insert(prefix.to_string(), text.clone());
            }
            other => panic!("{prefix}: translations must be strings, got {other}"),
        }
    }

    fn catalog(json: &str) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        flatten("", &serde_json::from_str(json).expect("valid locale JSON"), &mut out);
        out
    }

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split("%{")
            .skip(1)
            .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn all_languages_have_the_same_keys_and_placeholders() {
        let (en, de) = (catalog(EN), catalog(DE));
        let missing_de: Vec<_> = en.keys().filter(|k| !de.contains_key(*k)).collect();
        let extra_de: Vec<_> = de.keys().filter(|k| !en.contains_key(*k)).collect();
        assert!(missing_de.is_empty(), "missing in de.json: {missing_de:?}");
        assert!(extra_de.is_empty(), "not in en.json: {extra_de:?}");
        for (key, text) in &en {
            assert_eq!(
                placeholders(text),
                placeholders(&de[key]),
                "placeholders differ for {key}"
            );
        }
    }

    #[test]
    fn keys_used_in_code_exist() {
        let en = catalog(EN);
        let sources = [
            include_str!("agent/mod.rs"),
            include_str!("app.rs"),
            include_str!("commands.rs"),
            include_str!("config/settings.rs"),
            include_str!("config/watcher.rs"),
            include_str!("config/paths.rs"),
            include_str!("desktop/hotkey.rs"),
            include_str!("desktop/tray.rs"),
            include_str!("integrations/audio.rs"),
            include_str!("integrations/media.rs"),
            include_str!("integrations/radio.rs"),
            include_str!("integrations/spotify.rs"),
            include_str!("launcher/mod.rs"),
            include_str!("llm/ollama.rs"),
            include_str!("secrets/catalog.rs"),
            include_str!("secrets/store.rs"),
            include_str!("tools/args.rs"),
            include_str!("tools/builtin.rs"),
            include_str!("tools/executor.rs"),
            include_str!("tools/manifest.rs"),
            include_str!("tools/registry.rs"),
            include_str!("tools/template.rs"),
            include_str!("tools/trigger.rs"),
        ];
        let mut unknown = Vec::new();
        for source in sources {
            let mut rest = source;
            while let Some(index) = rest.find("t!(\"") {
                // Word boundary: skip e.g. `format!("`.
                let preceded_by_ident = rest[..index]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
                let after = &rest[index + 4..];
                if !preceded_by_ident {
                    let key = after.split('"').next().unwrap_or_default();
                    if !en.contains_key(key) {
                        unknown.push(key.to_string());
                    }
                }
                rest = after;
            }
        }
        assert!(
            unknown.is_empty(),
            "keys used in code but missing in en.json: {unknown:?}"
        );
    }
}
