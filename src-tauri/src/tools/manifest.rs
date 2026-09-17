use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::builtin::Builtin;
use super::template::{self, Context};
use super::trigger::{Trigger, TriggerSpec};

/// One tool, as written in `tools/**/*.json`. See `assets/schemas/tool.schema.json`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolManifest {
    /// Accepted (and ignored) so files can reference the JSON schema for editor support.
    #[serde(rename = "$schema", default)]
    #[allow(dead_code)]
    pub schema: Option<String>,
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub permission: Permission,
    #[serde(default)]
    pub parameters: Parameters,
    /// Phrases that run the tool directly, without the model (see `trigger.rs`).
    #[serde(default)]
    pub triggers: Vec<TriggerSpec>,
    pub action: Action,
}

/// Whether a tool runs immediately or asks the user first.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Auto,
    #[default]
    Confirm,
}

/// JSON-Schema subset accepted for tool parameters (flat object of scalar values).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    #[serde(rename = "type", default = "object_type")]
    pub kind: String,
    #[serde(default)]
    pub properties: BTreeMap<String, ParamSpec>,
    #[serde(default)]
    pub required: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParamSpec {
    #[serde(rename = "type")]
    pub kind: ParamType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum", default, skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    String,
    Number,
    Integer,
    Boolean,
}

/// What happens when the tool is called.
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    /// Opens a URL or URI (`https:`, `ms-settings:`, `spotify:` …) with the default handler.
    OpenUrl { url: String },
    /// Starts a program directly (no shell). Empty rendered arguments are dropped.
    Launch {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<String>,
    },
    /// Runs a fixed PowerShell script. Parameters arrive as `$env:ORBIT_ARG_<NAME>`.
    Powershell {
        script: String,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
    /// Performs an HTTP request and returns the response body to the model.
    Http {
        #[serde(default)]
        method: HttpMethod,
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
    /// A function implemented inside Orbit.
    Builtin { id: Builtin },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
}

impl HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
        }
    }
}

impl Action {
    pub fn kind(&self) -> &'static str {
        match self {
            Action::OpenUrl { .. } => "open_url",
            Action::Launch { .. } => "launch",
            Action::Powershell { .. } => "powershell",
            Action::Http { .. } => "http",
            Action::Builtin { .. } => "builtin",
        }
    }
}

impl ToolManifest {
    /// Semantic checks that the JSON schema alone cannot express.
    pub fn validate(&self) -> Result<(), String> {
        if !is_valid_tool_name(&self.name) {
            return Err(format!(
                "\"name\" \"{}\" ist ungültig: nur a-z, 0-9 und _, beginnend mit einem Buchstaben (2–64 Zeichen).",
                self.name
            ));
        }
        if self.description.trim().is_empty() {
            return Err(
                "\"description\" darf nicht leer sein – das Modell entscheidet anhand dieser Beschreibung.".into(),
            );
        }
        self.validate_parameters()?;
        self.validate_action()?;
        self.compile_triggers().map(|_| ())
    }

    pub fn compile_triggers(&self) -> Result<Vec<Trigger>, String> {
        self.triggers
            .iter()
            .map(|spec| spec.compile(&self.parameters.properties))
            .collect()
    }

    /// Names of all secrets this tool needs (template references and builtin requirements).
    pub fn secrets(&self) -> Vec<String> {
        let mut names: Vec<String> = match &self.action {
            Action::OpenUrl { url } => template::secret_names(url),
            Action::Launch { program, args, cwd } => std::iter::once(program)
                .chain(args)
                .chain(cwd)
                .flat_map(|t| template::secret_names(t))
                .collect(),
            Action::Powershell { .. } => Vec::new(),
            Action::Http { url, headers, body, .. } => std::iter::once(url)
                .chain(headers.values())
                .chain(body)
                .flat_map(|t| template::secret_names(t))
                .collect(),
            Action::Builtin { id } => id.required_secrets().iter().map(|s| s.to_string()).collect(),
        };
        names.sort();
        names.dedup();
        names
    }

    fn validate_parameters(&self) -> Result<(), String> {
        let params = &self.parameters;
        if params.kind != "object" {
            return Err("\"parameters.type\" muss \"object\" sein.".into());
        }
        for name in params.properties.keys() {
            if !is_valid_param_name(name) {
                return Err(format!(
                    "Parameter \"{name}\" ist ungültig: nur Buchstaben, Ziffern und _ erlaubt."
                ));
            }
        }
        for name in &params.required {
            if !params.properties.contains_key(name) {
                return Err(format!(
                    "\"required\" enthält \"{name}\", der Parameter ist aber nicht definiert."
                ));
            }
        }
        Ok(())
    }

    fn validate_action(&self) -> Result<(), String> {
        let known = |text: &str, context: Context| template::check(text, context, &self.parameters.properties);

        match &self.action {
            Action::OpenUrl { url } => known(url, Context::Url),
            Action::Launch { program, args, cwd } => {
                known(program, Context::Arg)?;
                args.iter().try_for_each(|a| known(a, Context::Arg))?;
                cwd.as_deref().map_or(Ok(()), |c| known(c, Context::Arg))
            }
            Action::Powershell { script, .. } => {
                if script.contains("{{") {
                    return Err("PowerShell-Skripte unterstützen keine {{Platzhalter}}. \
                        Nutze stattdessen $env:ORBIT_ARG_<NAME> (sicher gegen Code-Injection)."
                        .into());
                }
                Ok(())
            }
            Action::Http { url, headers, body, .. } => {
                let lower = url.to_ascii_lowercase();
                if !(lower.starts_with("http://") || lower.starts_with("https://")) {
                    return Err("\"action.url\" muss mit http:// oder https:// beginnen.".into());
                }
                known(url, Context::Url)?;
                headers.values().try_for_each(|h| known(h, Context::Header))?;
                body.as_deref().map_or(Ok(()), |b| known(b, Context::Body))
            }
            Action::Builtin { id } => {
                for required in id.required_params() {
                    if !self.parameters.properties.contains_key(*required) {
                        return Err(format!(
                            "Builtin \"{}\" braucht den Parameter \"{required}\".",
                            id.as_str()
                        ));
                    }
                }
                Ok(())
            }
        }
    }
}

fn is_valid_tool_name(name: &str) -> bool {
    let mut chars = name.chars();
    (2..=64).contains(&name.len())
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_valid_param_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn default_true() -> bool {
    true
}

fn object_type() -> String {
    "object".into()
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            kind: object_type(),
            properties: BTreeMap::new(),
            required: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Result<ToolManifest, String> {
        let manifest: ToolManifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
        manifest.validate()?;
        Ok(manifest)
    }

    #[test]
    fn all_default_tools_are_valid() {
        for (path, content) in crate::config::defaults::TOOLS {
            parse(content).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
    }

    #[test]
    fn minimal_tool_parses_with_safe_defaults() {
        let tool = parse(
            r#"{ "name": "hello", "description": "Sagt hallo", "action": { "type": "open_url", "url": "https://example.com" } }"#,
        )
        .unwrap();
        assert!(tool.enabled);
        assert_eq!(tool.permission, Permission::Confirm);
    }

    #[test]
    fn rejects_unknown_placeholder() {
        let err = parse(
            r#"{ "name": "t1", "description": "x", "action": { "type": "open_url", "url": "https://x.de/{{nope}}" } }"#,
        )
        .unwrap_err();
        assert!(err.contains("nope"), "{err}");
    }

    #[test]
    fn rejects_placeholders_in_powershell() {
        let err = parse(
            r#"{ "name": "t1", "description": "x",
                 "parameters": { "type": "object", "properties": { "a": { "type": "string" } } },
                 "action": { "type": "powershell", "script": "echo {{a}}" } }"#,
        )
        .unwrap_err();
        assert!(err.contains("ORBIT_ARG_"), "{err}");
    }

    #[test]
    fn rejects_unknown_fields_in_action() {
        assert!(parse(
            r#"{ "name": "t1", "description": "x", "action": { "type": "open_url", "url": "https://x.de", "oops": 1 } }"#
        )
        .is_err());
    }

    #[test]
    fn collects_secrets_from_templates_and_builtins() {
        let http = parse(
            r#"{ "name": "t1", "description": "x", "action": { "type": "http",
                 "url": "https://x.de/?k={{secret:api_key}}", "headers": { "X-Other": "{{secret:other_key}}" } } }"#,
        )
        .unwrap();
        assert_eq!(http.secrets(), vec!["api_key", "other_key"]);

        let spotify = parse(
            r#"{ "name": "t2", "description": "x",
                 "parameters": { "type": "object", "properties": { "query": { "type": "string" } } },
                 "action": { "type": "builtin", "id": "spotify_play" } }"#,
        )
        .unwrap();
        assert_eq!(spotify.secrets(), vec!["spotify_client_id", "spotify_client_secret"]);
    }

    #[test]
    fn invalid_trigger_is_reported() {
        let err = parse(
            r#"{ "name": "t1", "description": "x", "triggers": ["öffne {nope}"],
                 "action": { "type": "builtin", "id": "datetime" } }"#,
        )
        .unwrap_err();
        assert!(err.contains("nope"), "{err}");
    }

    #[test]
    fn rejects_invalid_names() {
        for name in ["A", "Hello", "1abc", "with-dash"] {
            let json = format!(
                r#"{{ "name": "{name}", "description": "x", "action": {{ "type": "builtin", "id": "datetime" }} }}"#
            );
            assert!(parse(&json).is_err(), "{name} should be rejected");
        }
    }
}
