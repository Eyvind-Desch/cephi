//! Minimal `{{param}}` templating with context-aware escaping.
//!
//! | Context  | Default filter | Used for                  |
//! |----------|----------------|---------------------------|
//! | `Url`    | `url`          | `open_url.url`, `http.url`|
//! | `Arg`    | `raw`          | `launch.program/args/cwd` |
//! | `Header` | `raw`          | `http.headers` (no CR/LF) |
//! | `Body`   | `json`         | `http.body`               |
//!
//! Explicit filters: `{{name|raw}}`, `{{name|url}}`, `{{name|json}}`.

use std::collections::BTreeMap;

use serde_json::Value;

use super::args::Args;
use super::manifest::ParamSpec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Url,
    Arg,
    Header,
    Body,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    Raw,
    Url,
    Json,
}

impl Context {
    fn default_filter(self) -> Filter {
        match self {
            Context::Url => Filter::Url,
            Context::Arg | Context::Header => Filter::Raw,
            Context::Body => Filter::Json,
        }
    }
}

enum Segment<'a> {
    Literal(&'a str),
    Placeholder { name: &'a str, filter: Filter },
}

fn parse(template: &str, context: Context) -> Result<Vec<Segment<'_>>, String> {
    let mut segments = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        segments.push(Segment::Literal(&rest[..start]));
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| format!("Nicht geschlossener Platzhalter in \"{template}\"."))?;
        let inner = &after[..end];
        let (name, filter) = match inner.split_once('|') {
            Some((name, filter)) => (name.trim(), parse_filter(filter.trim())?),
            None => (inner.trim(), context.default_filter()),
        };
        if name.is_empty() {
            return Err(format!("Leerer Platzhalter in \"{template}\"."));
        }
        segments.push(Segment::Placeholder { name, filter });
        rest = &after[end + 2..];
    }
    segments.push(Segment::Literal(rest));
    Ok(segments)
}

fn parse_filter(filter: &str) -> Result<Filter, String> {
    match filter {
        "raw" => Ok(Filter::Raw),
        "url" => Ok(Filter::Url),
        "json" => Ok(Filter::Json),
        other => Err(format!("Unbekannter Filter \"{other}\" (erlaubt: raw, url, json).")),
    }
}

/// Load-time check: syntax is valid and every placeholder refers to a declared parameter.
pub fn check(template: &str, context: Context, params: &BTreeMap<String, ParamSpec>) -> Result<(), String> {
    for segment in parse(template, context)? {
        if let Segment::Placeholder { name, .. } = segment {
            if !params.contains_key(name) {
                return Err(format!(
                    "Platzhalter {{{{{name}}}}} verweist auf keinen definierten Parameter."
                ));
            }
        }
    }
    Ok(())
}

/// Renders a template with validated arguments. Missing optional values render as empty.
pub fn render(template: &str, context: Context, args: &Args) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    for segment in parse(template, context)? {
        match segment {
            Segment::Literal(text) => out.push_str(text),
            Segment::Placeholder { name, filter } => {
                let value = args.get(name);
                let rendered = match filter {
                    Filter::Raw => plain(value),
                    Filter::Url => url::form_urlencoded::byte_serialize(plain(value).as_bytes()).collect(),
                    Filter::Json => serde_json::to_string(value.unwrap_or(&Value::Null)).unwrap_or_default(),
                };
                if context == Context::Header && rendered.contains(['\r', '\n']) {
                    return Err(format!(
                        "Parameter \"{name}\" enthält Zeilenumbrüche (nicht erlaubt in Headern)."
                    ));
                }
                out.push_str(&rendered);
            }
        }
    }
    Ok(out)
}

/// Literal text before the first placeholder, e.g. `"ms-settings:"` for `"ms-settings:{{page}}"`.
pub fn literal_prefix(template: &str) -> &str {
    template.find("{{").map_or(template, |i| &template[..i])
}

fn plain(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(value: Value) -> Args {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn url_context_encodes_by_default() {
        let a = args(json!({ "q": "a&b c" }));
        assert_eq!(
            render("https://x.de/?q={{q}}", Context::Url, &a).unwrap(),
            "https://x.de/?q=a%26b+c"
        );
    }

    #[test]
    fn explicit_filters() {
        let a = args(json!({ "u": "https://x.de/a?b=1", "n": 5 }));
        assert_eq!(render("{{ u | raw }}", Context::Url, &a).unwrap(), "https://x.de/a?b=1");
        assert_eq!(
            render(r#"{"n": {{n}}, "u": {{u}}}"#, Context::Body, &a).unwrap(),
            r#"{"n": 5, "u": "https://x.de/a?b=1"}"#
        );
    }

    #[test]
    fn missing_optional_renders_empty() {
        assert_eq!(render("a{{x}}b", Context::Arg, &Args::new()).unwrap(), "ab");
    }

    #[test]
    fn header_rejects_newlines() {
        let a = args(json!({ "h": "x\r\nEvil: 1" }));
        assert!(render("{{h}}", Context::Header, &a).is_err());
    }

    #[test]
    fn syntax_errors() {
        assert!(parse("{{open", Context::Arg).is_err());
        assert!(parse("{{ }}", Context::Arg).is_err());
        assert!(parse("{{a|shout}}", Context::Arg).is_err());
    }

    #[test]
    fn prefix() {
        assert_eq!(literal_prefix("ms-settings:{{page}}"), "ms-settings:");
        assert_eq!(literal_prefix("{{url}}"), "");
    }
}
