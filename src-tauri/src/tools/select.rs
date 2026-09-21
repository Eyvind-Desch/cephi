//! Picks the tools a request needs when there are too many to send all.
//!
//! Every tool definition costs context: Cephi's 19 tools are about 1500 tokens, which a small local
//! model has to read before it writes a single word. Above a limit, only the tools whose name, title
//! or description share words with the request are offered, plus the ones already used in this chat
//! (for follow-ups like "and close it again").

use std::cmp::Reverse;
use std::collections::HashSet;

/// At most this many tools per request for local models; cloud models get all of them up to
/// [`CLOUD_LIMIT`].
pub const LOCAL_LIMIT: usize = 8;
pub const CLOUD_LIMIT: usize = 64;

/// Words shorter than this carry no meaning for matching ("a", "to", "in", "der", …).
const MIN_WORD: usize = 3;
/// Words match when they share this many leading characters ("issues" ↔ "issue", "Dateien" ↔ "Datei").
const STEM: usize = 5;

/// What a tool is matched by. `context` is the MCP server or the tool's title in the user's
/// language — the strongest hint, because that is the word people actually use ("Websuche").
pub struct Candidate<'a> {
    pub name: &'a str,
    pub context: &'a str,
    pub description: &'a str,
}

/// The tools to send, in their original order. `used` holds the names called earlier in the chat.
pub fn relevant<T>(
    tools: Vec<T>,
    describe: impl Fn(&T) -> Candidate<'_>,
    request: &str,
    used: &HashSet<String>,
    limit: usize,
) -> Vec<T> {
    if tools.len() <= limit {
        return tools;
    }
    let words = words(request);
    let scored: Vec<(usize, usize, T)> = tools
        .into_iter()
        .enumerate()
        .map(|(index, tool)| (score(&describe(&tool), &words, used), index, tool))
        .filter(|(score, _, _)| *score > 0)
        .collect();

    // Tools from the chat history must not fill every slot: a long chat would then crowd out the
    // tool the current request is actually about.
    let (history, mut matching): (Vec<_>, Vec<_>) = scored.into_iter().partition(|(s, _, _)| *s >= USED_SCORE);
    matching.sort_by_key(|(score, _, _)| Reverse(*score));
    let for_request = (limit - limit / 2).min(matching.len());
    let from_history = history.len().min(limit - for_request);

    let mut chosen: Vec<(usize, usize, T)> = history.into_iter().take(from_history).collect();
    chosen.extend(matching.into_iter().take(limit - from_history));
    // Back into catalog order, which is what the model sees.
    chosen.sort_by_key(|(_, index, _)| *index);
    chosen.into_iter().map(|(_, _, tool)| tool).collect()
}

/// Score of a tool that was already used in this chat; higher than any word match can reach.
const USED_SCORE: usize = 100;

fn score(tool: &Candidate<'_>, request: &[String], used: &HashSet<String>) -> usize {
    if used.contains(tool.name) {
        return USED_SCORE;
    }
    let name = words(&tool.name.replace('_', " "));
    let context = words(&tool.context.replace(['_', '-'], " "));
    let description = words(tool.description);
    request
        .iter()
        .map(|word| {
            let hits = |list: &[String]| list.iter().any(|w| similar(w, word));
            // The title or server name ("Websuche", "github") says the most, then the tool name.
            4 * usize::from(hits(&context)) + 3 * usize::from(hits(&name)) + usize::from(hits(&description))
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

    /// `(name, context, description)` as both call sites hand it in.
    type Entry = (&'static str, &'static str, &'static str);

    fn describe(entry: &Entry) -> Candidate<'_> {
        Candidate {
            name: entry.0,
            context: entry.1,
            description: entry.2,
        }
    }

    fn mcp_catalog() -> Vec<Entry> {
        vec![
            ("github__create_issue", "github", "Create a new issue in a repository"),
            ("github__search_code", "github", "Search code across repositories"),
            ("github__list_pull_requests", "github", "List pull requests"),
            ("notion__search", "notion", "Search pages in the Notion workspace"),
            ("notion__create_page", "notion", "Create a page"),
            ("files__read_file", "files", "Read a file from disk"),
            ("files__list_directory", "files", "List the entries of a directory"),
        ]
    }

    /// Cephi's own tools: the context is the title in the user's language.
    fn own_catalog() -> Vec<Entry> {
        vec![
            ("web_search", "Websuche", "Searches the web and returns the top results with short excerpts."),
            ("translate", "Übersetzen", "Translates text between German and English."),
            ("spotify_play", "Spotify", "Plays music on Spotify."),
            ("media_control", "Lautstärke", "Playback and volume of the system."),
            ("open_app", "App öffnen", "Starts an installed application."),
            ("datetime", "Datum und Uhrzeit", "Returns the current date and time."),
            ("run_powershell", "PowerShell", "Runs a PowerShell command."),
            ("fetch_page", "Seite lesen", "Reads a web page and returns its text."),
            ("clipboard_read", "Zwischenablage", "Reads the clipboard."),
        ]
    }

    fn names(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|e| e.0).collect()
    }

    fn pick(catalog: Vec<Entry>, request: &str, limit: usize) -> Vec<Entry> {
        relevant(catalog, describe, request, &HashSet::new(), limit)
    }

    #[test]
    fn few_tools_are_all_sent() {
        assert_eq!(pick(mcp_catalog(), "hello", 10).len(), 7);
    }

    #[test]
    fn picks_by_context_and_tool_words() {
        let picked = pick(mcp_catalog(), "Create an issue on GitHub about the crash", 3);
        assert_eq!(names(&picked)[0], "github__create_issue");
        assert!(picked.iter().all(|e| e.1 == "github"), "{:?}", names(&picked));

        let picked = pick(mcp_catalog(), "lies die Datei notes.txt (read file)", 2);
        assert_eq!(names(&picked), ["files__read_file"]);
    }

    /// The German title is what a German request names, not the English description.
    #[test]
    fn own_tools_match_their_title() {
        assert!(names(&pick(own_catalog(), "mach eine websuche nach tauri", 3)).contains(&"web_search"));
        assert!(names(&pick(own_catalog(), "übersetze das bitte", 3)).contains(&"translate"));
        assert!(names(&pick(own_catalog(), "spiele etwas auf Spotify", 3)).contains(&"spotify_play"));
        assert!(names(&pick(own_catalog(), "welche uhrzeit haben wir?", 3)).contains(&"datetime"));
    }

    /// The keywords from the tool files are what makes German questions find English tools.
    #[test]
    fn german_questions_find_their_tool_through_keywords() {
        let catalog: Vec<Entry> = vec![
            ("system_info", "Systeminfo arbeitsspeicher ram cpu festplatte akku frei", "Returns CPU load and free memory."),
            ("weather", "Wetter vorhersage temperatur regen regnen schneit sonnig", "Current weather for a place."),
            ("datetime", "Datum & Uhrzeit uhrzeit heute wochentag", "Returns the current date and time."),
            ("lock_pc", "PC sperren bildschirm", "Locks the PC."),
            ("web_search", "Websuche internet nachrichten aktuell", "Searches the web."),
            ("translate", "Übersetzen übersetzung englisch deutsch", "Translates text."),
            ("run_powershell", "PowerShell-Befehl skript konsole", "Runs a PowerShell command."),
            ("clipboard_read", "Zwischenablage lesen kopiert", "Reads the clipboard."),
            ("open_explorer", "Explorer öffnen ordner dateien", "Opens the file explorer."),
        ];
        let asked = |question: &str, expected: &str| {
            let picked = relevant(catalog.clone(), describe, question, &HashSet::new(), 4);
            assert!(names(&picked).contains(&expected), "{question:?} → {:?}", names(&picked));
        };
        asked("Wie viel Arbeitsspeicher ist frei?", "system_info");
        asked("Wird es morgen regnen?", "weather");
        asked("Welchen Wochentag haben wir?", "datetime");
        asked("Sperr mal den PC", "lock_pc");
        asked("Was steht in meiner Zwischenablage?", "clipboard_read");
    }

    #[test]
    fn unrelated_requests_get_no_tools() {
        assert!(pick(mcp_catalog(), "wie spät ist es", 3).is_empty());
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
        let picked = relevant(mcp_catalog(), describe, "read the file notes.txt", &used, 4);
        assert!(names(&picked).contains(&"files__read_file"), "{:?}", names(&picked));
        assert_eq!(picked.len(), 4);
    }

    #[test]
    fn tools_used_earlier_stay_available() {
        let used = HashSet::from(["notion__create_page".to_string()]);
        let picked = relevant(mcp_catalog(), describe, "and now rename it", &used, 3);
        assert_eq!(names(&picked), ["notion__create_page"]);
    }
}
