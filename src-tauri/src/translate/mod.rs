//! Offline translation through a local LibreTranslate instance (`infra/compose.yaml`).
//!
//! Direction is automatic by default: text in the native language goes to the foreign language,
//! everything else to the native language (e.g. German → English, English → German).

mod libretranslate;

use rust_i18n::t;
use serde::Serialize;

use crate::config::TranslateSettings;

/// Mirrored in `src/lib/ipc.ts` (`Translation`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    /// The text that was translated, so the UI can show and reuse it.
    pub source: String,
    pub text: String,
    /// Other common translations (mostly for single words and short phrases).
    pub alternatives: Vec<String>,
    /// Language codes, e.g. `en` → `de`.
    pub from: String,
    pub to: String,
}

/// Translates `text`. `to` is a language code or name ("en", "Englisch", "english"); `None` = automatic.
pub async fn translate(
    http: &reqwest::Client,
    settings: &TranslateSettings,
    text: &str,
    to: Option<&str>,
) -> Result<Translation, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(t!("translate.empty").into());
    }
    let client = libretranslate::Client::new(http, &settings.libretranslate_url);

    let explicit = match to.map(str::trim).filter(|t| !t.is_empty() && *t != "auto") {
        Some(name) => Some(language_code(name).ok_or_else(|| t!("translate.unknown_language", language = name))?),
        None => None,
    };
    if let Some(target) = explicit {
        return client.translate(text, "auto", target).await.map(clean);
    }

    // Automatic: try the native language first; if the text already is in it, go the other way.
    let native = settings.native_language.as_str();
    let first = client.translate(text, "auto", native).await?;
    if first.from == native {
        return client.translate(text, native, &settings.foreign_language).await.map(clean);
    }
    Ok(clean(first))
}

/// Drops alternatives that repeat the translation.
fn clean(mut translation: Translation) -> Translation {
    let main = translation.text.to_lowercase();
    let mut seen = vec![main];
    translation.alternatives.retain(|alt| {
        let key = alt.trim().to_lowercase();
        let keep = !key.is_empty() && !seen.contains(&key);
        seen.push(key);
        keep
    });
    translation
}

/// Accepts codes and language names in English and German, including inflected forms ("ins Englische").
pub fn language_code(name: &str) -> Option<&'static str> {
    const LANGUAGES: &[(&str, &[&str])] = &[
        ("de", &["de", "deutsch", "deutsche", "deutschen", "german"]),
        ("en", &["en", "englisch", "englische", "englischen", "english"]),
        ("es", &["es", "spanisch", "spanische", "spanischen", "spanish"]),
        ("fr", &["fr", "französisch", "französische", "französischen", "french"]),
        ("it", &["it", "italienisch", "italienische", "italienischen", "italian"]),
    ];
    let name = name.trim().to_lowercase();
    LANGUAGES
        .iter()
        .find(|(_, names)| names.contains(&name.as_str()))
        .map(|(code, _)| *code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_names() {
        assert_eq!(language_code("Englische"), Some("en"));
        assert_eq!(language_code("german"), Some("de"));
        assert_eq!(language_code("klingonisch"), None);
    }

    #[test]
    fn duplicate_alternatives_are_dropped() {
        let translation = clean(Translation {
            source: "Kannst du mir helfen?".into(),
            text: "Can you help me?".into(),
            alternatives: vec!["can you help me?".into(), "Could you help me?".into(), "Could you help me?".into()],
            from: "de".into(),
            to: "en".into(),
        });
        assert_eq!(translation.alternatives, vec!["Could you help me?"]);
    }

    /// Needs the local LibreTranslate container (`infra/start.ps1`).
    #[tokio::test]
    #[ignore]
    async fn live_translation_picks_the_direction() {
        let http = reqwest::Client::new();
        let settings = TranslateSettings::default();
        let to_german = translate(&http, &settings, "nevertheless", None).await.unwrap();
        assert_eq!((to_german.from.as_str(), to_german.to.as_str()), ("en", "de"));
        let to_english = translate(&http, &settings, "Kannst du mir helfen?", None).await.unwrap();
        assert_eq!(to_english.to, "en");
    }
}
