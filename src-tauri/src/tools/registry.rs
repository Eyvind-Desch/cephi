use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::args::{self, Args};
use super::manifest::ToolManifest;
use crate::config::Diagnostic;

/// A loaded, validated and enabled tool.
pub struct Tool {
    pub manifest: ToolManifest,
    /// Path relative to the config root, for display.
    pub source: String,
}

impl Tool {
    pub fn title(&self) -> &str {
        self.manifest.title.as_deref().unwrap_or(&self.manifest.name)
    }

    pub fn normalize_args(&self, raw: &Value) -> Result<Args, String> {
        args::normalize(&self.manifest.parameters, raw)
    }

    /// Tool definition in the format expected by Ollama's `/api/chat`.
    fn definition(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.manifest.name,
                "description": self.manifest.description,
                "parameters": self.manifest.parameters,
            }
        })
    }
}

/// All enabled tools, keyed by name.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Tool>,
}

impl ToolRegistry {
    /// Loads every `*.json` below `dir` (recursively). Invalid files are skipped and reported.
    pub fn load_dir(root: &Path, dir: &Path) -> (Self, Vec<Diagnostic>) {
        let mut registry = Self::default();
        let mut diagnostics = Vec::new();

        let mut files = Vec::new();
        if let Err(e) = collect_json_files(dir, &mut files) {
            if dir.exists() {
                diagnostics.push(Diagnostic::for_path(root, dir, format!("Ordner nicht lesbar: {e}")));
            }
        }
        files.sort();

        for path in files {
            match load_file(&path) {
                Ok(manifest) if !manifest.enabled => {}
                Ok(manifest) => {
                    let source = Diagnostic::for_path(root, &path, "").source;
                    if let Some(existing) = registry.tools.get(&manifest.name) {
                        diagnostics.push(Diagnostic::for_path(
                            root,
                            &path,
                            format!(
                                "Tool-Name \"{}\" ist bereits in {} vergeben.",
                                manifest.name, existing.source
                            ),
                        ));
                        continue;
                    }
                    registry.tools.insert(manifest.name.clone(), Tool { manifest, source });
                }
                Err(message) => diagnostics.push(Diagnostic::for_path(root, &path, message)),
            }
        }

        (registry, diagnostics)
    }

    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Tool> {
        self.tools.values()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn definitions(&self) -> Vec<Value> {
        self.tools.values().map(Tool::definition).collect()
    }
}

fn load_file(path: &Path) -> Result<ToolManifest, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("Datei nicht lesbar: {e}"))?;
    let manifest: ToolManifest = serde_json::from_str(&text).map_err(|e| format!("Ungültiges Tool: {e}"))?;
    manifest.validate()?;
    Ok(manifest)
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("json")) {
            out.push(path);
        }
    }
    Ok(())
}
