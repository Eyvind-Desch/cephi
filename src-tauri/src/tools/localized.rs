//! Text in config files that can be a plain string or translated per language.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `"Weather"` or `{ "en": "Weather", "de": "Wetter" }`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LocalizedText {
    Plain(String),
    Translated(BTreeMap<String, String>),
}

impl LocalizedText {
    pub fn plain(text: &str) -> Self {
        Self::Plain(text.to_string())
    }

    pub fn translated(entries: &[(&str, &str)]) -> Self {
        Self::Translated(entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    /// Text for `locale`, falling back to English, then to any available translation.
    pub fn resolve(&self, locale: &str) -> &str {
        match self {
            Self::Plain(text) => text,
            Self::Translated(map) => map
                .get(locale)
                .or_else(|| map.get("en"))
                .or_else(|| map.values().next())
                .map_or("", String::as_str),
        }
    }

    pub fn validate(&self, field: &str) -> Result<(), String> {
        let empty = match self {
            Self::Plain(text) => text.trim().is_empty(),
            Self::Translated(map) => map.is_empty() || map.values().any(|v| v.trim().is_empty()),
        };
        if empty {
            Err(rust_i18n::t!("config.empty_text", field = field).into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_with_fallbacks() {
        let text: LocalizedText = serde_json::from_str(r#"{ "en": "Weather", "de": "Wetter" }"#).unwrap();
        assert_eq!(text.resolve("de"), "Wetter");
        assert_eq!(text.resolve("fr"), "Weather");

        let only_de = LocalizedText::translated(&[("de", "Wetter")]);
        assert_eq!(only_de.resolve("en"), "Wetter");

        let plain: LocalizedText = serde_json::from_str(r#""Weather""#).unwrap();
        assert_eq!(plain.resolve("de"), "Weather");
    }

    #[test]
    fn rejects_empty_text() {
        assert!(LocalizedText::plain(" ").validate("title").is_err());
        assert!(LocalizedText::translated(&[]).validate("title").is_err());
        assert!(LocalizedText::plain("Ok").validate("title").is_ok());
    }
}
