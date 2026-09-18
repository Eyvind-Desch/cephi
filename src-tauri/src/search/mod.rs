//! Web search through a local SearXNG instance (`infra/searxng`).
//!
//! SearXNG queries Google, Bing, Brave, DuckDuckGo, … at once and returns deduplicated results
//! as JSON. Cephi shows them as cards and hands short excerpts to the model.

mod searxng;

use std::fmt::Write;

use serde::Serialize;

pub use searxng::search;

/// Characters per excerpt sent to the model. Short excerpts keep small local models fast.
const MODEL_SNIPPET_CHARS: usize = 320;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub engines: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResponse {
    pub results: Vec<WebResult>,
    /// Direct answers and infobox summaries (e.g. from Wikipedia), if any.
    pub answers: Vec<String>,
}

impl SearchResponse {
    /// Numbered excerpts for the model. Model-facing, so English.
    pub fn for_model(&self, query: &str) -> String {
        // Small models tend to describe the request ("The user asked about …") instead of answering.
        let mut text = format!(
            "Web results for \"{query}\".\n\
             Answer directly in the user's language, in 1-4 sentences, using only these results. \
             If the query is just a word or name, explain what it is or means. \
             Cite sources as [1], [2]. Never talk about \"the user\", the query or the results themselves. \
             If the results do not contain the answer, say so briefly.\n"
        );
        for answer in &self.answers {
            let _ = write!(text, "\nDirect answer: {}\n", excerpt(answer, MODEL_SNIPPET_CHARS));
        }
        for (index, result) in self.results.iter().enumerate() {
            let _ = write!(
                text,
                "\n[{}] {}\n{}\n{}\n",
                index + 1,
                result.title,
                result.url,
                excerpt(&result.snippet, MODEL_SNIPPET_CHARS)
            );
        }
        text
    }
}

/// Collapses whitespace and cuts at a word boundary.
pub fn excerpt(text: &str, max_chars: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max_chars {
        return collapsed;
    }
    let cut: String = collapsed.chars().take(max_chars).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{cut} …")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excerpts_collapse_whitespace_and_cut_at_words() {
        assert_eq!(excerpt("a \n  b", 10), "a b");
        assert_eq!(excerpt("hello wonderful world", 12), "hello …");
        assert_eq!(excerpt("äöü äöü", 5), "äöü …");
    }

    #[test]
    fn model_text_numbers_results() {
        let response = SearchResponse {
            results: vec![
                WebResult {
                    title: "First".into(),
                    url: "https://a.example".into(),
                    snippet: "one".into(),
                    engines: vec![],
                },
                WebResult {
                    title: "Second".into(),
                    url: "https://b.example".into(),
                    snippet: "two".into(),
                    engines: vec![],
                },
            ],
            answers: vec![],
        };
        let text = response.for_model("q");
        assert!(text.contains("[1] First\nhttps://a.example\none"));
        assert!(text.contains("[2] Second"));
    }
}
