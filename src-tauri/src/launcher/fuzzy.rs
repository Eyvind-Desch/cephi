//! Small, predictable fuzzy matcher for app names.
//!
//! Scores (0–100) are tiered so thresholds are meaningful:
//! exact 100 · prefix 90 · word prefix 80 · all words prefix-match 75 · substring 65 ·
//! in-order subsequence ≤ 50.

pub const EXACT: u32 = 100;

pub fn score(query: &str, candidate: &str) -> u32 {
    let query = normalize(query);
    let name = normalize(candidate);
    if query.is_empty() || name.is_empty() {
        return 0;
    }

    let base = if name == query {
        EXACT
    } else if name.starts_with(&query) {
        90
    } else if words(&name).any(|w| w.starts_with(&query)) {
        80
    } else if words(&query).all(|q| words(&name).any(|w| w.starts_with(q))) {
        75
    } else if name.contains(&query) {
        65
    } else {
        subsequence_score(&query, &name)
    };

    // Prefer the plain app over "App Uninstall", "App (Beta)" etc.
    let extra_words = words(&name).count().saturating_sub(words(&query).count()) as u32;
    let penalty = if is_auxiliary(&name) { 30 } else { extra_words.min(5) };
    base.saturating_sub(penalty)
}

fn subsequence_score(query: &str, name: &str) -> u32 {
    let mut remaining = name.chars();
    let matched = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .all(|c| remaining.any(|n| n == c));
    if !matched {
        return 0;
    }
    let ratio = query.chars().count() as f32 / name.chars().count().max(1) as f32;
    (20.0 + 30.0 * ratio.min(1.0)) as u32
}

fn normalize(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(' ').filter(|w| !w.is_empty())
}

fn is_auxiliary(name: &str) -> bool {
    [
        "uninstall",
        "deinstall",
        "readme",
        "help",
        "hilfe",
        "documentation",
        "release notes",
    ]
    .iter()
    .any(|word| name.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers() {
        assert_eq!(score("spotify", "Spotify"), 100);
        assert!(score("visual", "Visual Studio Code") >= 85);
        assert!(score("code", "Visual Studio Code") >= 75);
        assert!(
            score("vs code", "Visual Studio Code") < 75,
            "not all words are prefixes"
        );
        assert!(score("vis stu", "Visual Studio Code") >= 70);
        assert!(score("potif", "Spotify") >= 60);
        assert_eq!(score("xyz", "Spotify"), 0);
    }

    #[test]
    fn prefers_main_app_over_uninstaller() {
        assert!(score("steam", "Steam") > score("steam", "Uninstall Steam"));
        assert!(score("discord", "Discord") > score("discord", "Discord PTB Beta"));
    }

    #[test]
    fn ignores_case_and_punctuation() {
        assert_eq!(score("  EXCEL ", "Excel"), 100);
        assert_eq!(score("paint.net", "Paint NET"), 100);
    }
}
