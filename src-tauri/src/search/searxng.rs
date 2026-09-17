//! SearXNG JSON API client: `GET /search?q=…&format=json`.

use std::time::Duration;

use rust_i18n::t;
use serde::Deserialize;
use serde_json::Value;

use super::{SearchResponse, WebResult};
use crate::config::SearchSettings;

/// SearXNG itself waits at most `max_request_timeout` (4 s) for the engines.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Deserialize)]
struct RawResponse {
    #[serde(default)]
    results: Vec<RawResult>,
    #[serde(default)]
    answers: Vec<Value>,
    #[serde(default)]
    infoboxes: Vec<RawInfobox>,
}

#[derive(Deserialize)]
struct RawResult {
    #[serde(default)]
    title: String,
    url: Option<String>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    engines: Vec<String>,
}

#[derive(Deserialize)]
struct RawInfobox {
    #[serde(default)]
    content: String,
}

pub async fn search(http: &reqwest::Client, settings: &SearchSettings, query: &str) -> Result<SearchResponse, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err(t!("search.empty_query").into());
    }

    let base = settings.searxng_url.trim_end_matches('/');
    let response = http
        .get(format!("{base}/search"))
        .query(&[("q", query), ("format", "json"), ("language", settings.language.as_str())])
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() || e.is_timeout() {
                t!("search.unreachable", url = base).to_string()
            } else {
                t!("search.failed", error = e.without_url()).to_string()
            }
        })?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        // SearXNG answers 403 when the JSON format is not enabled in settings.yml.
        return Err(t!("search.json_disabled").into());
    }
    if !status.is_success() {
        return Err(t!("search.failed", error = status).into());
    }
    let raw: RawResponse = response
        .json()
        .await
        .map_err(|e| t!("search.failed", error = e.without_url()).to_string())?;

    let parsed = parse(raw, settings.max_results);
    if parsed.results.is_empty() && parsed.answers.is_empty() {
        return Err(t!("search.no_results", query = query).into());
    }
    Ok(parsed)
}

fn parse(raw: RawResponse, max_results: usize) -> SearchResponse {
    let results = raw
        .results
        .into_iter()
        .filter_map(|r| {
            let url = r.url.filter(|u| u.starts_with("http://") || u.starts_with("https://"))?;
            Some(WebResult {
                title: super::excerpt(if r.title.trim().is_empty() { &url } else { &r.title }, 160),
                snippet: super::excerpt(&r.content, 600),
                url,
                engines: r.engines,
            })
        })
        .take(max_results)
        .collect();

    // Answers are plain strings in older SearXNG versions and objects in newer ones.
    let answers = raw
        .answers
        .iter()
        .filter_map(|a| a.as_str().or_else(|| a["answer"].as_str()))
        .map(str::to_string)
        .chain(raw.infoboxes.into_iter().map(|i| i.content))
        .filter(|a| !a.trim().is_empty())
        .take(2)
        .collect();

    SearchResponse { results, answers }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_results_answers_and_infoboxes() {
        let raw: RawResponse = serde_json::from_str(
            r#"{
              "results": [
                { "title": "Tauri", "url": "https://tauri.app", "content": "Build  apps", "engines": ["google"] },
                { "title": "", "url": "https://example.com/x", "content": "" },
                { "title": "No URL" },
                { "title": "Bad scheme", "url": "javascript:alert(1)" },
                { "title": "Over limit", "url": "https://c.example" }
              ],
              "answers": ["42", { "answer": "Berlin" }],
              "infoboxes": [{ "content": "A framework" }]
            }"#,
        )
        .unwrap();
        let parsed = parse(raw, 2);
        assert_eq!(parsed.results.len(), 2);
        assert_eq!(parsed.results[0].snippet, "Build apps");
        assert_eq!(parsed.results[1].title, "https://example.com/x");
        assert_eq!(parsed.answers, vec!["42", "Berlin"]);
    }

    /// Needs the local SearXNG container (`infra/searxng/start.ps1`).
    #[tokio::test]
    #[ignore]
    async fn live_search() {
        let response = search(&reqwest::Client::new(), &SearchSettings::default(), "tauri framework")
            .await
            .unwrap();
        assert!(!response.results.is_empty());
    }
}
