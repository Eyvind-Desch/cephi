//! Known services and their keys, so the keys view can group them with labels and help.
//!
//! - AI providers come from `settings.providers` and are always listed.
//! - Integrations (e.g. Spotify) are listed when a tool uses them or a key is stored.
//! - Secrets used by tools but not known here appear in the "custom" group.

use std::collections::BTreeMap;

use rust_i18n::t;
use serde::Serialize;

use super::store;
use crate::config::{ProviderKind, ProviderSettings, Settings};

struct SecretSpec {
    name: String,
    label: String,
    /// Hidden input (secrets) vs. plain input (IDs).
    sensitive: bool,
    placeholder: String,
}

struct GroupSpec {
    id: String,
    title: String,
    description: String,
    /// Icon name understood by the frontend (also selects the brand colour).
    icon: String,
    docs_url: Option<String>,
    /// Whether "test connection" is supported (see `app::test_secret_group`).
    testable: bool,
    /// Listed even when no key is stored and nothing uses it yet.
    always_show: bool,
    secrets: Vec<SecretSpec>,
}

/// Built at call time so texts follow the active language.
fn known_groups(settings: &Settings) -> Vec<GroupSpec> {
    let mut groups: Vec<GroupSpec> = settings.providers.iter().map(provider_group).collect();
    groups.push(GroupSpec {
        id: "spotify".into(),
        title: "Spotify".into(),
        description: t!("secret_groups.spotify.description").into(),
        icon: "music".into(),
        docs_url: Some("https://developer.spotify.com/dashboard".into()),
        testable: true,
        always_show: false,
        secrets: vec![
            SecretSpec {
                name: "spotify_client_id".into(),
                label: "Client ID".into(),
                sensitive: false,
                placeholder: t!("secret_groups.spotify.client_id_placeholder").into(),
            },
            SecretSpec {
                name: "spotify_client_secret".into(),
                label: "Client Secret".into(),
                sensitive: true,
                placeholder: t!("secret_groups.spotify.client_secret_placeholder").into(),
            },
        ],
    });
    groups
}

fn provider_group(provider: &ProviderSettings) -> GroupSpec {
    let base_url = provider.base_url.as_deref().unwrap_or_default();
    let (icon, docs_url) = if provider.kind == ProviderKind::Anthropic {
        ("anthropic", Some("https://console.anthropic.com/settings/keys"))
    } else if base_url.contains("generativelanguage.googleapis.com") {
        ("gemini", Some("https://aistudio.google.com/apikey"))
    } else if base_url.contains("api.openai.com") {
        ("openai", Some("https://platform.openai.com/api-keys"))
    } else if base_url.contains("openrouter.ai") {
        ("ai", Some("https://openrouter.ai/settings/keys"))
    } else if base_url.contains("groq.com") {
        ("ai", Some("https://console.groq.com/keys"))
    } else {
        ("ai", None)
    };

    GroupSpec {
        id: provider.id.clone(),
        title: provider.display_name(),
        description: t!("secret_groups.ai.description").into(),
        icon: icon.into(),
        docs_url: docs_url.map(Into::into),
        testable: true,
        always_show: true,
        secrets: vec![SecretSpec {
            name: provider.api_key_secret(),
            label: "API Key".into(),
            sensitive: true,
            placeholder: t!("secret_groups.ai.placeholder").into(),
        }],
    }
}

const CUSTOM_GROUP_ID: &str = "custom";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretGroupInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub docs_url: Option<String>,
    pub testable: bool,
    pub secrets: Vec<SecretInfo>,
}

/// A single secret. The value itself is never exposed.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretInfo {
    pub name: String,
    pub label: String,
    pub sensitive: bool,
    pub placeholder: String,
    pub is_set: bool,
    /// Tools and profiles that use this secret.
    pub used_by: Vec<String>,
}

/// All groups to show: AI providers first, then integrations in use, then custom tool keys.
pub fn overview(tool_usage: &BTreeMap<String, Vec<String>>, settings: &Settings) -> Vec<SecretGroupInfo> {
    overview_with(tool_usage, settings, store::is_set)
}

fn overview_with(
    tool_usage: &BTreeMap<String, Vec<String>>,
    settings: &Settings,
    is_set: impl Fn(&str) -> bool,
) -> Vec<SecretGroupInfo> {
    // Profiles "use" their provider's key.
    let mut usage = tool_usage.clone();
    let locale = rust_i18n::locale();
    for profile in &settings.profiles {
        if let Some(provider) = settings.provider(&profile.provider) {
            usage
                .entry(provider.api_key_secret())
                .or_default()
                .push(profile.label.resolve(&locale).to_string());
        }
    }

    let known = known_groups(settings);
    let is_known = |name: &str| known.iter().any(|g| g.secrets.iter().any(|s| s.name == name));

    let custom: Vec<SecretInfo> = usage
        .iter()
        .filter(|(name, _)| !is_known(name))
        .map(|(name, used_by)| SecretInfo {
            name: name.clone(),
            label: name.clone(),
            sensitive: true,
            placeholder: t!("secret_groups.custom.placeholder").into(),
            is_set: is_set(name),
            used_by: used_by.clone(),
        })
        .collect();

    let mut groups: Vec<SecretGroupInfo> = known
        .into_iter()
        .filter_map(|group| {
            let secrets: Vec<SecretInfo> = group
                .secrets
                .into_iter()
                .map(|spec| SecretInfo {
                    is_set: is_set(&spec.name),
                    used_by: usage.get(&spec.name).cloned().unwrap_or_default(),
                    name: spec.name,
                    label: spec.label,
                    sensitive: spec.sensitive,
                    placeholder: spec.placeholder,
                })
                .collect();
            let relevant = group.always_show || secrets.iter().any(|s| s.is_set || !s.used_by.is_empty());
            relevant.then_some(SecretGroupInfo {
                id: group.id,
                title: group.title,
                description: group.description,
                icon: group.icon,
                docs_url: group.docs_url,
                testable: group.testable,
                secrets,
            })
        })
        .collect();

    if !custom.is_empty() {
        groups.push(SecretGroupInfo {
            id: CUSTOM_GROUP_ID.into(),
            title: t!("secret_groups.custom.title").into(),
            description: t!("secret_groups.custom.description").into(),
            icon: "wrench".into(),
            docs_url: None,
            testable: false,
            secrets: custom,
        });
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<String>> {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (secret, tool) in entries {
            map.entry(secret.to_string()).or_default().push(tool.to_string());
        }
        map
    }

    fn no_providers() -> Settings {
        Settings {
            providers: Vec::new(),
            profiles: Settings::default()
                .profiles
                .into_iter()
                .filter(|p| p.provider == crate::llm::LOCAL_PROVIDER_ID)
                .collect(),
            ..Settings::default()
        }
    }

    #[test]
    fn groups_known_services_and_custom_keys() {
        let groups = overview_with(
            &usage(&[
                ("spotify_client_id", "Spotify"),
                ("spotify_client_secret", "Spotify"),
                ("deepl_key", "Translate"),
            ]),
            &no_providers(),
            |name| name == "spotify_client_id",
        );
        assert_eq!(
            groups.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
            ["spotify", "custom"]
        );
        assert_eq!(groups[0].secrets.len(), 2);
        assert!(groups[0].secrets[0].is_set && !groups[0].secrets[1].is_set);
        assert_eq!(groups[1].secrets[0].name, "deepl_key");
    }

    #[test]
    fn unused_integrations_without_keys_are_hidden() {
        assert!(overview_with(&BTreeMap::new(), &no_providers(), |_| false).is_empty());
        // A stored key keeps its service visible even if no tool uses it anymore.
        assert_eq!(
            overview_with(&BTreeMap::new(), &no_providers(), |name| name
                == "spotify_client_secret")
            .len(),
            1
        );
    }

    #[test]
    fn ai_providers_are_always_listed_first_with_their_profiles() {
        let groups = overview_with(&BTreeMap::new(), &Settings::default(), |_| false);
        let ids: Vec<&str> = groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["anthropic", "gemini", "openai"]);
        assert_eq!(groups[0].secrets[0].name, "anthropic_api_key");
        assert_eq!(groups[0].secrets[0].used_by, ["Claude"]);
        assert_eq!(groups[0].icon, "anthropic");
        assert!(groups[2].secrets[0].used_by.is_empty());
    }
}
