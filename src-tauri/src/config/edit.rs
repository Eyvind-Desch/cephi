//! Writes changes from the settings page back into the JSON files.
//!
//! Only the touched keys change: the file is read as JSON, patched and written back with its key order,
//! so hand edits and settings the page doesn't know about stay as they are. Nothing is written unless the
//! result is valid, and the file is replaced atomically.

use std::fs;
use std::path::Path;

use rust_i18n::t;
use serde_json::{Map, Value};

use super::defaults;
use super::settings::Settings;
use crate::mcp::config::{self as mcp_config, McpConfig};

/// Merges `patch` into `settings.json`. Objects merge key by key; `null` removes a key.
pub fn update_settings(path: &Path, patch: &Value) -> Result<(), String> {
    let mut document = read_object(path, defaults::SETTINGS)?;
    merge(&mut document, patch);
    let settings: Settings =
        serde_json::from_value(document.clone()).map_err(|e| t!("config.invalid_json", error = e).to_string())?;
    settings.validate()?;
    write(path, &document)
}

/// Adds or replaces an MCP server. `previous` is its old name when it was renamed.
pub fn save_mcp_server(path: &Path, name: &str, previous: Option<&str>, server: &Value) -> Result<(), String> {
    let mut document = read_object(path, defaults::MCP)?;
    let servers = servers_mut(&mut document)?;
    if let Some(previous) = previous.filter(|p| *p != name) {
        servers.shift_remove(previous);
    }
    servers.insert(name.to_string(), clean(server.clone()));
    validate_mcp(&document)?;
    write(path, &document)
}

pub fn remove_mcp_server(path: &Path, name: &str) -> Result<(), String> {
    let mut document = read_object(path, defaults::MCP)?;
    servers_mut(&mut document)?.shift_remove(name);
    write(path, &document)
}

fn servers_mut(document: &mut Value) -> Result<&mut Map<String, Value>, String> {
    let root = document
        .as_object_mut()
        .ok_or_else(|| t!("config.invalid_json", error = "not an object").to_string())?;
    root.entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| t!("config.invalid_json", error = "mcpServers is not an object").to_string())
}

fn validate_mcp(document: &Value) -> Result<(), String> {
    let config: McpConfig =
        serde_json::from_value(document.clone()).map_err(|e| t!("config.invalid_json", error = e).to_string())?;
    mcp_config::validate(&config)
}

/// The form sends empty fields; they don't belong in the file.
fn clean(mut value: Value) -> Value {
    if let Value::Object(map) = &mut value {
        map.retain(|_, v| match v {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            Value::Bool(b) => *b,
            Value::Number(_) => true,
        });
    }
    value
}

fn merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                if value.is_null() {
                    target.shift_remove(key);
                } else {
                    merge(target.entry(key.clone()).or_insert(Value::Null), value);
                }
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

/// The file's content, or `default` if it doesn't exist yet.
fn read_object(path: &Path, default: &str) -> Result<Value, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => default.to_string(),
        Err(e) => return Err(t!("config.file_unreadable", error = e).into()),
    };
    serde_json::from_str(&text).map_err(|e| t!("config.invalid_json", error = e).to_string())
}

fn write(path: &Path, document: &Value) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(document).map_err(|e| e.to_string())?;
    text.push('\n');
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, text)
        .and_then(|()| fs::rename(&temp, path))
        .map_err(|e| t!("config.write_failed", error = e).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn new(content: &str) -> Self {
            let path = std::env::temp_dir().join(format!("cephi-edit-test-{}.json", uuid::Uuid::new_v4()));
            fs::write(&path, content).unwrap();
            Self(path)
        }
        fn json(&self) -> Value {
            serde_json::from_str(&fs::read_to_string(&self.0).unwrap()).unwrap()
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn settings_patch_keeps_other_keys_and_order() {
        let file = TempFile::new(r#"{ "hotkey": "Ctrl+Alt+Space", "window": { "margin": 16, "hideOnBlur": true } }"#);
        update_settings(
            &file.0,
            &serde_json::json!({ "window": { "hideOnBlur": false }, "language": "de" }),
        )
        .unwrap();
        let json = file.json();
        assert_eq!(json["window"]["margin"], 16);
        assert_eq!(json["window"]["hideOnBlur"], false);
        let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["hotkey", "window", "language"]);
    }

    #[test]
    fn invalid_settings_are_not_written() {
        let original = r#"{ "hotkey": "Ctrl+Alt+Space" }"#;
        let file = TempFile::new(original);
        assert!(update_settings(&file.0, &serde_json::json!({ "defaultProfile": "nope" })).is_err());
        assert_eq!(fs::read_to_string(&file.0).unwrap(), original);
    }

    #[test]
    fn mcp_servers_are_added_renamed_and_removed() {
        let file = TempFile::new(r#"{ "$schema": "./schemas/mcp.schema.json", "mcpServers": {} }"#);
        let server =
            serde_json::json!({ "command": "npx", "args": ["-y", "x"], "env": {}, "url": "", "disabled": false });
        save_mcp_server(&file.0, "files", None, &server).unwrap();
        assert_eq!(
            file.json()["mcpServers"]["files"],
            serde_json::json!({ "command": "npx", "args": ["-y", "x"] })
        );

        save_mcp_server(&file.0, "docs", Some("files"), &server).unwrap();
        assert!(file.json()["mcpServers"].get("files").is_none());
        assert!(file.json()["mcpServers"].get("docs").is_some());

        let broken = serde_json::json!({ "command": "npx", "url": "https://x.example" });
        assert!(save_mcp_server(&file.0, "bad", None, &broken).is_err());

        remove_mcp_server(&file.0, "docs").unwrap();
        assert_eq!(file.json()["mcpServers"], serde_json::json!({}));
        assert_eq!(file.json()["$schema"], "./schemas/mcp.schema.json");
    }
}
