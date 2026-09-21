//! Picks the MCP tools a request needs when there are too many to send all.
//!
//! Every tool definition costs context, and small local models pick worse the more tools they see.
//! Above a limit, only the tools whose name, server or description share words with the request
//! are offered, plus the ones already used in this chat (for follow-ups like "and close it again").

use std::cmp::Reverse;
use std::collections::HashSet;

use super::McpTool;

/// At most this many MCP tools per request for local models; cloud models get all of them up to
/// [`CLOUD_LIMIT`].
pub const LOCAL_LIMIT: usize = 8;
pub const CLOUD_LIMIT: usize = 64;

/// Words shorter than this carry no meaning for matching ("a", "to", "in", "der", …).
const MIN_WORD: usize = 3;
/// Words match when they share this many leading characters ("issues" ↔ "issue", "Dateien" ↔ "Datei").
const STEM: usize = 5;

/// The tools to send, in their original order. `used` holds exposed names called earlier in the chat.
pub fn relevant(tools: Vec<McpTool>, request: &str, used: &HashSet<String>, limit: usize) -> Vec<McpTool> {
    if tools.len() <= limit {
        return tools;
    }
    let words = words(request);
    let scored: Vec<(usize, usize, McpTool)> = tools
        .into_iter()
        .enumerate()
        .map(|(index, tool)| (score(&tool, &words, used), index, tool))
        .filter(|(score, _, _)| *score > 0)
        .collect();

    // Tools from the chat history must not fill every slot: a long chat would then crowd out the
    // tool the current request is actually about.
    let (history, mut matching): (Vec<_>, Vec<_>) = scored.into_iter().partition(|(s, _, _)| *s >= USED_SCORE);
    matching.sort_by_key(|(score, _, _)| Reverse(*score));
    let for_request = (limit - limit / 2).min(matching.len());
    let from_history = history.len().min(limit - for_request);

    let mut chosen: Vec<(usize, usize, McpTool)> = history.into_iter().take(from_history).collect();
    chosen.extend(matching.into_iter().take(limit - from_history));
    // Back into catalog order, which is what the model sees.
    chosen.sort_by_key(|(_, index, _)| *index);
    chosen.into_iter().map(|(_, _, tool)| tool).collect()
}

/// Score of a tool that was already used in this chat; higher than any word match can reach.
const USED_SCORE: usize = 100;

fn score(tool: &McpTool, request: &[String], used: &HashSet<String>) -> usize {
    if used.contains(&tool.exposed) {
        return USED_SCORE;
    }
    let name = words(&tool.name.replace('_', " "));
    let server = words(&tool.server.replace(['_', '-'], " "));
    let description = words(&tool.description);
    request
        .iter()
        .map(|word| {
            let hits = |list: &[String]| list.iter().any(|w| similar(w, word));
            // The server name ("github", "notion") says the most, then the tool name.
            4 * usize::from(hits(&server)) + 3 * usize::from(hits(&name)) + usize::from(hits(&description))
        })
        .sum()
}

fn similar(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let stem = |w: &str| w.chars().take(STEM).collect::<String>();
    a.chars().count() >= STEM && b.chars().count() >= STEM && stem(a) == stem(b)
}

/// Frequent English and German words that would match almost every description.
const STOP_WORDS: [&str; 40] = [
    "the", "and", "for", "with", "from", "this", "that", "into", "about", "your", "you", "are", "can", "all", "new",
    "not", "der", "die", "das", "und", "ein", "eine", "einen", "mit", "von", "für", "auf", "ist", "wie", "was",
    "bitte", "mir", "mich", "den", "dem", "des", "aus", "oder", "nicht", "ich",
];

fn words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= MIN_WORD)
        .map(str::to_lowercase)
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect();
    words.sort();
    words.dedup();
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(server: &str, name: &str, description: &str) -> McpTool {
        McpTool {
            server: server.into(),
            name: name.into(),
            exposed: format!("{server}__{name}"),
            title: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({}),
            auto_approved: false,
        }
    }

    fn catalog() -> Vec<McpTool> {
        vec![
            tool("github", "create_issue", "Create a new issue in a repository"),
            tool("github", "search_code", "Search code across repositories"),
            tool("github", "list_pull_requests", "List pull requests"),
            tool("notion", "search", "Search pages in the Notion workspace"),
            tool("notion", "create_page", "Create a page"),
            tool("files", "read_file", "Read a file from disk"),
            tool("files", "list_directory", "List the entries of a directory"),
        ]
    }

    fn names(tools: &[McpTool]) -> Vec<&str> {
        tools.iter().map(|t| t.exposed.as_str()).collect()
    }

    #[test]
    fn few_tools_are_all_sent() {
        let all = relevant(catalog(), "hello", &HashSet::new(), 10);
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn picks_by_server_and_tool_words() {
        let picked = relevant(
            catalog(),
            "Create an issue on GitHub about the crash",
            &HashSet::new(),
            3,
        );
        assert_eq!(names(&picked)[0], "github__create_issue");
        assert!(picked.iter().all(|t| t.server == "github"), "{:?}", names(&picked));

        let picked = relevant(catalog(), "lies die Datei notes.txt (read file)", &HashSet::new(), 2);
        assert_eq!(names(&picked), ["files__read_file"]);
    }

    #[test]
    fn unrelated_requests_get_no_mcp_tools() {
        assert!(relevant(catalog(), "wie spät ist es", &HashSet::new(), 3).is_empty());
    }

    #[test]
    fn the_chat_history_does_not_crowd_out_the_fitting_tool() {
        // Four tools used earlier, but the request is about a fifth one.
        let used: HashSet<String> = [
            "github__create_issue",
            "github__search_code",
            "github__list_pull_requests",
            "notion__search",
        ]
        .iter()
        .map(|name| (*name).to_string())
        .collect();
        let picked = relevant(catalog(), "read the file notes.txt", &used, 4);
        assert!(names(&picked).contains(&"files__read_file"), "{:?}", names(&picked));
        assert_eq!(picked.len(), 4);
    }

    #[test]
    fn tools_used_earlier_stay_available() {
        let used = HashSet::from(["notion__create_page".to_string()]);
        let picked = relevant(catalog(), "and now rename it", &used, 3);
        assert_eq!(names(&picked), ["notion__create_page"]);
    }
}
