//! Fast-path triggers: phrases that run a tool directly, without asking the model.
//!
//! Pattern syntax (case-insensitive, whole input must match, trailing `.!?` ignored):
//!
//! | Syntax        | Meaning                                  |
//! |---------------|------------------------------------------|
//! | `{name}`      | captures text into parameter `name`      |
//! | `(a|b|c)`     | one of the alternatives                  |
//! | `[text]`      | optional                                 |
//! | whitespace    | one or more spaces                       |
//!
//! Example: `(spiele|play) {query} (auf|in) spotify`

use std::collections::BTreeMap;

use regex::Regex;
use rust_i18n::t;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::manifest::ParamSpec;

/// A trigger as written in the manifest: a pattern string, or a pattern with fixed arguments.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum TriggerSpec {
    Pattern(String),
    WithArgs {
        pattern: String,
        #[serde(default)]
        args: Map<String, Value>,
    },
}

#[derive(Clone, Debug)]
pub struct Trigger {
    regex: Regex,
    captures: Vec<String>,
    fixed_args: Map<String, Value>,
    /// Minimum number of literal characters an input must contain to match.
    /// More specific triggers are tried first.
    pub specificity: usize,
}

impl TriggerSpec {
    pub fn compile(&self, params: &BTreeMap<String, ParamSpec>) -> Result<Trigger, String> {
        let (pattern, fixed_args) = match self {
            TriggerSpec::Pattern(pattern) => (pattern.as_str(), Map::new()),
            TriggerSpec::WithArgs { pattern, args } => (pattern.as_str(), args.clone()),
        };
        let context = |e: String| t!("triggers.error", pattern = pattern, error = e).to_string();

        for key in fixed_args.keys() {
            if !params.contains_key(key) {
                return Err(context(t!("triggers.unknown_arg", name = key).into()));
            }
        }

        let mut parser = Parser {
            chars: pattern.trim().chars().collect(),
            pos: 0,
            captures: Vec::new(),
        };
        let (body, min_literals) = parser.sequence(&[]).map_err(context)?;
        if parser.pos != parser.chars.len() {
            return Err(context(
                t!("triggers.unexpected", char = parser.chars[parser.pos]).into(),
            ));
        }
        for name in &parser.captures {
            if !params.contains_key(name) {
                return Err(context(t!("triggers.unknown_param", name = name).into()));
            }
        }
        if min_literals == 0 {
            return Err(context(t!("triggers.needs_literal").into()));
        }

        let regex = Regex::new(&format!(r"(?i)^\s*{body}\s*[.!?]*\s*$"))
            .map_err(|e| context(t!("triggers.invalid_regex", error = e).into()))?;

        Ok(Trigger {
            regex,
            captures: parser.captures,
            fixed_args,
            specificity: min_literals,
        })
    }
}

impl Trigger {
    /// Returns the tool arguments if `input` matches.
    pub fn matches(&self, input: &str) -> Option<Value> {
        let caps = self.regex.captures(input)?;
        let mut args = self.fixed_args.clone();
        for name in &self.captures {
            if let Some(value) = caps.name(name) {
                let value = value.as_str().trim().trim_matches(['"', '\'', '„', '“', '”']).trim();
                if value.is_empty() {
                    return None;
                }
                args.insert(name.clone(), Value::String(value.to_string()));
            }
        }
        Some(Value::Object(args))
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    captures: Vec<String>,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) -> bool {
        let start = self.pos;
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
        self.pos > start
    }

    /// Parses until one of `stop` (not consumed) or the end.
    /// Returns the regex and the minimum number of literal characters it requires.
    fn sequence(&mut self, stop: &[char]) -> Result<(String, usize), String> {
        let mut out = String::new();
        let mut min_literals = 0;
        while let Some(c) = self.peek() {
            if stop.contains(&c) {
                break;
            }
            match c {
                c if c.is_whitespace() => {
                    self.skip_whitespace();
                    if self.peek() == Some('[') {
                        // " [x]" → optional including its leading space
                        let inner = self.optional()?;
                        out.push_str(&format!(r"(?:\s+{inner})?"));
                    } else if self.peek().is_some_and(|c| !stop.contains(&c)) {
                        out.push_str(r"\s+");
                    }
                }
                '[' => {
                    let inner = self.optional()?;
                    if self.skip_whitespace() {
                        // "[x] " → optional including its trailing space
                        out.push_str(&format!(r"(?:{inner}\s+)?"));
                    } else {
                        out.push_str(&format!("(?:{inner})?"));
                    }
                }
                '(' => {
                    self.pos += 1;
                    let mut alternatives = vec![self.sequence(&['|', ')'])?];
                    while self.peek() == Some('|') {
                        self.pos += 1;
                        alternatives.push(self.sequence(&['|', ')'])?);
                    }
                    self.expect(')')?;
                    min_literals += alternatives.iter().map(|(_, n)| *n).min().unwrap_or(0);
                    let regexes: Vec<String> = alternatives.into_iter().map(|(regex, _)| regex).collect();
                    out.push_str(&format!("(?:{})", regexes.join("|")));
                }
                '{' => {
                    self.pos += 1;
                    let start = self.pos;
                    while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                        self.pos += 1;
                    }
                    let name: String = self.chars[start..self.pos].iter().collect();
                    self.expect('}')?;
                    if name.is_empty() {
                        return Err(t!("triggers.empty_capture").into());
                    }
                    if self.captures.contains(&name) {
                        return Err(t!("triggers.duplicate_capture", name = name).into());
                    }
                    out.push_str(&format!("(?P<{name}>.+?)"));
                    self.captures.push(name);
                }
                ')' | ']' | '|' | '}' => return Err(t!("triggers.unexpected", char = c).into()),
                _ => {
                    self.pos += 1;
                    min_literals += 1;
                    out.push_str(&regex::escape(&c.to_string()));
                }
            }
        }
        Ok((out, min_literals))
    }

    /// Optional parts contribute no required literals.
    fn optional(&mut self) -> Result<String, String> {
        self.expect('[')?;
        let (inner, _) = self.sequence(&[']'])?;
        self.expect(']')?;
        Ok(inner)
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(t!("triggers.missing_char", char = c).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params(names: &[&str]) -> BTreeMap<String, ParamSpec> {
        names
            .iter()
            .map(|n| {
                (
                    n.to_string(),
                    serde_json::from_value(json!({ "type": "string" })).unwrap(),
                )
            })
            .collect()
    }

    fn compile(pattern: &str) -> Trigger {
        TriggerSpec::Pattern(pattern.into())
            .compile(&params(&["query", "name", "kind"]))
            .unwrap()
    }

    #[test]
    fn spotify_phrases() {
        let t = compile("(spiele|spiel|play) {query} (auf|in) spotify");
        assert_eq!(
            t.matches("Spiele Another Love von Tom Odell in Spotify"),
            Some(json!({ "query": "Another Love von Tom Odell" }))
        );
        assert_eq!(
            t.matches("play „Mr. Brightside“ auf spotify!"),
            Some(json!({ "query": "Mr. Brightside" }))
        );
        assert_eq!(t.matches("spiele in spotify"), None);
        assert_eq!(t.matches("spielen wir was auf spotify"), None);
    }

    #[test]
    fn optional_parts_absorb_spaces() {
        let t = compile("(öffne|starte) [die] [app] {name}");
        assert_eq!(t.matches("öffne die app Discord"), Some(json!({ "name": "Discord" })));
        assert_eq!(t.matches("starte Discord"), Some(json!({ "name": "Discord" })));

        let t = compile("{name} [bitte] öffnen");
        assert_eq!(t.matches("Steam bitte öffnen"), Some(json!({ "name": "Steam" })));
        assert_eq!(t.matches("Steam öffnen"), Some(json!({ "name": "Steam" })));
    }

    #[test]
    fn short_prefix_needs_a_space() {
        let t = compile("g {query}");
        assert_eq!(t.matches("g rust tauri"), Some(json!({ "query": "rust tauri" })));
        assert_eq!(t.matches("google"), None);
    }

    #[test]
    fn fixed_args_are_merged() {
        let spec: TriggerSpec =
            serde_json::from_value(json!({ "pattern": "spiele [das] album {query}", "args": { "kind": "album" } }))
                .unwrap();
        let t = spec.compile(&params(&["query", "kind"])).unwrap();
        assert_eq!(
            t.matches("spiele das Album Divide"),
            Some(json!({ "query": "Divide", "kind": "album" }))
        );
    }

    #[test]
    fn literal_regex_characters_are_escaped() {
        let t = compile("was ist {query}?");
        assert!(t.matches("was ist 2+2?").is_some());
        assert!(compile("c++ {query}").matches("c++ docs").is_some());
    }

    #[test]
    fn specificity_counts_required_literals_only() {
        assert_eq!(compile("(spiele|play) {query} (auf|in) spotify").specificity, 4 + 2 + 7);
        assert_eq!(compile("(öffne|start) [die] [app] {name}").specificity, 5);
    }

    #[test]
    fn invalid_patterns() {
        let p = params(&["query"]);
        for bad in ["{unknown} x", "(a|b", "[x", "x)", "{query}", "a {query} {query}", "{}"] {
            assert!(
                TriggerSpec::Pattern(bad.into()).compile(&p).is_err(),
                "{bad} should fail"
            );
        }
    }
}
