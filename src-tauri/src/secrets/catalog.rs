//! Known services and their keys, so the keys view can group them with labels and help.
//!
//! To add a service (e.g. an AI provider), add a [`GroupSpec`] in [`known_groups`] and its texts
//! to the locale files. Secrets used by tools but not listed here appear in the "custom" group.

use std::collections::BTreeMap;

use rust_i18n::t;
use serde::Serialize;

use super::store;

struct SecretSpec {
    name: &'static str,
    label: String,
    /// Hidden input (secrets) vs. plain input (IDs).
    sensitive: bool,
    placeholder: String,
}

struct GroupSpec {
    id: &'static str,
    title: String,
    description: String,
    /// Icon name understood by the frontend.
    icon: &'static str,
    docs_url: Option<&'static str>,
    /// Whether "test connection" is supported (see `app::test_secret_group`).
    testable: bool,
    secrets: Vec<SecretSpec>,
}

/// Built at call time so texts follow the active language.
fn known_groups() -> Vec<GroupSpec> {
    vec![GroupSpec {
        id: "spotify",
        title: "Spotify".into(),
        description: t!("secret_groups.spotify.description").into(),
        icon: "music",
        docs_url: Some("https://developer.spotify.com/dashboard"),
        testable: true,
        secrets: vec![
            SecretSpec {
                name: "spotify_client_id",
                label: "Client ID".into(),
                sensitive: false,
                placeholder: t!("secret_groups.spotify.client_id_placeholder").into(),
            },
            SecretSpec {
                name: "spotify_client_secret",
                label: "Client Secret".into(),
                sensitive: true,
                placeholder: t!("secret_groups.spotify.client_secret_placeholder").into(),
            },
        ],
    }]
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
    /// Titles of the tools that use this secret.
    pub used_by: Vec<String>,
}

/// Groups relevant right now: known services that a tool uses or that already have a stored key,
/// followed by keys of custom tools.
pub fn overview(usage: &BTreeMap<String, Vec<String>>) -> Vec<SecretGroupInfo> {
    overview_with(usage, store::is_set)
}

fn overview_with(usage: &BTreeMap<String, Vec<String>>, is_set: impl Fn(&str) -> bool) -> Vec<SecretGroupInfo> {
    let known = known_groups();
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
                    name: spec.name.into(),
                    label: spec.label,
                    sensitive: spec.sensitive,
                    placeholder: spec.placeholder,
                    is_set: is_set(spec.name),
                    used_by: usage.get(spec.name).cloned().unwrap_or_default(),
                })
                .collect();
            let relevant = secrets.iter().any(|s| s.is_set || !s.used_by.is_empty());
            relevant.then(|| SecretGroupInfo {
                id: group.id.into(),
                title: group.title,
                description: group.description,
                icon: group.icon.into(),
                docs_url: group.docs_url.map(Into::into),
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

    #[test]
    fn groups_known_services_and_custom_keys() {
        let groups = overview_with(
            &usage(&[
                ("spotify_client_id", "Spotify"),
                ("spotify_client_secret", "Spotify"),
                ("deepl_key", "Translate"),
            ]),
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
    fn unused_services_without_keys_are_hidden() {
        assert!(overview_with(&BTreeMap::new(), |_| false).is_empty());
        // A stored key keeps its service visible even if no tool uses it anymore.
        assert_eq!(
            overview_with(&BTreeMap::new(), |name| name == "spotify_client_secret").len(),
            1
        );
    }
}
