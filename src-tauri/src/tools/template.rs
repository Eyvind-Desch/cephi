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
//!
//! `{{secret:name}}` inserts a secret from the Windows Credential Manager (see [`crate::secrets`]).

use std::collections::BTreeMap;

use rust_i18n::t;
use serde_json::Value;

use super::args::Args;
use super::manifest::ParamSpec;
use crate::secrets;

const SECRET_PREFIX: &str = "secret:";
const MASK: &str = "••••••";

/// How `{{secret:…}}` placeholders are rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Secrets {
    /// Real values, for execution.
    Resolve,
    /// Masked, for previews shown to the user.
    Mask,
}

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
            .ok_or_else(|| t!("templates.unclosed", template = template).to_string())?;
        let inner = &after[..end];
        let (name, filter) = match inner.split_once('|') {
            Some((name, filter)) => (name.trim(), parse_filter(filter.trim())?),
            None => (inner.trim(), context.default_filter()),
        };
        if name.is_empty() {
            return Err(t!("templates.empty", template = template).into());
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
        other => Err(t!("templates.unknown_filter", filter = other).into()),
    }
}

/// Load-time check: syntax is valid and every placeholder refers to a declared parameter
/// or a validly named secret.
pub fn check(template: &str, context: Context, params: &BTreeMap<String, ParamSpec>) -> Result<(), String> {
    for segment in parse(template, context)? {
        if let Segment::Placeholder { name, .. } = segment {
            if let Some(secret) = name.strip_prefix(SECRET_PREFIX) {
                if !secrets::is_valid_name(secret) {
                    return Err(t!("secrets.invalid_name", name = secret).into());
                }
            } else if !params.contains_key(name) {
                return Err(t!("templates.unknown_param", name = name).into());
            }
        }
    }
    Ok(())
}

/// Names of all secrets referenced by a template.
pub fn secret_names(template: &str) -> Vec<String> {
    parse(template, Context::Arg)
        .map(|segments| {
            segments
                .into_iter()
                .filter_map(|segment| match segment {
                    Segment::Placeholder { name, .. } => name.strip_prefix(SECRET_PREFIX).map(str::to_string),
                    Segment::Literal(_) => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Renders a template with validated arguments. Missing optional values render as empty.
pub fn render(template: &str, context: Context, args: &Args, secrets_mode: Secrets) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    for segment in parse(template, context)? {
        match segment {
            Segment::Literal(text) => out.push_str(text),
            Segment::Placeholder { name, filter } => {
                let secret_value;
                let value = match name.strip_prefix(SECRET_PREFIX) {
                    Some(secret) => {
                        secret_value = Value::String(match secrets_mode {
                            Secrets::Mask => MASK.to_string(),
                            Secrets::Resolve => {
                                secrets::get(secret)?.ok_or_else(|| t!("secrets.missing", name = secret).to_string())?
                            }
                        });
                        Some(&secret_value)
                    }
                    None => args.get(name),
                };
                let rendered = match filter {
                    Filter::Raw => plain(value),
                    Filter::Url => url::form_urlencoded::byte_serialize(plain(value).as_bytes()).collect(),
                    Filter::Json => serde_json::to_string(value.unwrap_or(&Value::Null)).unwrap_or_default(),
                };
                if context == Context::Header && rendered.contains(['\r', '\n']) {
                    return Err(t!("templates.header_newline", name = name).into());
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

    fn render_resolved(template: &str, context: Context, args: &Args) -> Result<String, String> {
        render(template, context, args, Secrets::Resolve)
    }

    #[test]
    fn url_context_encodes_by_default() {
        let a = args(json!({ "q": "a&b c" }));
        assert_eq!(
            render_resolved("https://x.de/?q={{q}}", Context::Url, &a).unwrap(),
            "https://x.de/?q=a%26b+c"
        );
    }

    #[test]
    fn explicit_filters() {
        let a = args(json!({ "u": "https://x.de/a?b=1", "n": 5 }));
        assert_eq!(
            render_resolved("{{ u | raw }}", Context::Url, &a).unwrap(),
            "https://x.de/a?b=1"
        );
        assert_eq!(
            render_resolved(r#"{"n": {{n}}, "u": {{u}}}"#, Context::Body, &a).unwrap(),
            r#"{"n": 5, "u": "https://x.de/a?b=1"}"#
        );
    }

    #[test]
    fn missing_optional_renders_empty() {
        assert_eq!(render_resolved("a{{x}}b", Context::Arg, &Args::new()).unwrap(), "ab");
    }

    #[test]
    fn header_rejects_newlines() {
        let a = args(json!({ "h": "x\r\nEvil: 1" }));
        assert!(render_resolved("{{h}}", Context::Header, &a).is_err());
    }

    #[test]
    fn secrets_are_masked_resolved_and_listed() {
        let _serial = crate::secrets::test_support::CREDENTIAL_MANAGER
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Unique name: the test touches the real Credential Manager, runs must not interfere.
        let name = format!("cephi_test_{}", uuid::Uuid::new_v4().simple());
        let template = format!("Bearer {{{{secret:{name}}}}}");
        assert_eq!(secret_names(&template), vec![name.clone()]);
        assert_eq!(
            render(&template, Context::Header, &Args::new(), Secrets::Mask).unwrap(),
            format!("Bearer {MASK}")
        );
        assert!(render_resolved(&template, Context::Header, &Args::new())
            .unwrap_err()
            .contains(&name));

        secrets::set(&name, "abc123").unwrap();
        let resolved = render_resolved(&template, Context::Header, &Args::new());
        secrets::delete(&name).unwrap();
        assert_eq!(resolved.unwrap(), "Bearer abc123");
    }

    #[test]
    fn secret_names_are_validated() {
        let params = BTreeMap::new();
        assert!(check("{{secret:api_key}}", Context::Header, &params).is_ok());
        assert!(check("{{secret:Bad-Key}}", Context::Header, &params).is_err());
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
