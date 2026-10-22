use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::defaults;
use crate::error::{AppError, AppResult};

/// Environment variable to override the config folder (useful for development).
const CONFIG_DIR_ENV: &str = "CEPHI_CONFIG_DIR";
/// Remembers which default tools were offered and what was written, see [`sync_default_tools`].
const SEEDED_TOOLS_FILE: &str = ".seeded-tools.json";

/// Locations of all user-editable configuration files.
///
/// Default root: `%APPDATA%\Cephi`
/// ```text
/// Cephi/
/// ├── settings.json
/// ├── tools/          one JSON file per tool, subfolders allowed
/// └── schemas/        JSON schemas for editor autocompletion (regenerated on start)
/// ```
#[derive(Clone, Debug)]
pub struct ConfigPaths {
    pub root: PathBuf,
    pub settings: PathBuf,
    pub tools: PathBuf,
    pub schemas: PathBuf,
    /// MCP servers, same format as Claude Desktop.
    pub mcp: PathBuf,
}

impl ConfigPaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let root = match std::env::var_os(CONFIG_DIR_ENV) {
            Some(dir) => PathBuf::from(dir),
            None => app
                .path()
                .config_dir()
                .map_err(|e| AppError::msg(format!("config directory not found: {e}")))?
                .join("Cephi"),
        };
        Ok(Self::from_root(root))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self {
            settings: root.join("settings.json"),
            tools: root.join("tools"),
            schemas: root.join("schemas"),
            mcp: root.join("mcp.json"),
            root,
        }
    }

    /// Creates the folder structure and seeds defaults.
    ///
    /// - Schemas are always rewritten so they match the running version.
    /// - `settings.json` is created only if missing.
    /// - Default tools: see [`sync_default_tools`].
    pub fn ensure_initialized(&self) -> io::Result<()> {
        fs::create_dir_all(&self.schemas)?;
        for (name, content) in defaults::SCHEMAS {
            fs::write(self.schemas.join(name), content)?;
        }

        if !self.settings.exists() {
            fs::write(&self.settings, defaults::SETTINGS)?;
            log::info!("seeded {}", self.settings.display());
        }
        if !self.mcp.exists() {
            fs::write(&self.mcp, defaults::MCP)?;
        }

        sync_default_tools(&self.root.join(SEEDED_TOOLS_FILE), &self.tools, defaults::TOOLS)
    }
}

/// What Cephi last wrote for each default tool: `path → content hash`.
/// `None` = the user owns the file (it existed before, or was edited before hashes were recorded).
#[derive(Default, Deserialize, Serialize)]
struct SeedRecord {
    tools: BTreeMap<String, Option<String>>,
}

/// Keeps default tools current without ever overwriting user changes:
///
/// - New defaults (e.g. from an app update) are created.
/// - A default the user never edited is updated when the app ships a new version of it.
/// - Edited or deleted defaults are left alone for good.
fn sync_default_tools(record_path: &Path, tools_dir: &Path, defaults: &[(&str, &str)]) -> io::Result<()> {
    let mut record = load_record(record_path);
    let (mut added, mut updated) = (0, 0);

    for (relative, content) in defaults {
        let path = tools_dir.join(relative);
        let current = fs::read_to_string(&path).ok();
        let default_hash = content_hash(content);

        match (record.tools.get(*relative), current) {
            // Never offered before.
            (None, None) => {
                write_new(&path, content)?;
                record.tools.insert(relative.to_string(), Some(default_hash));
                added += 1;
            }
            // A file identical to the default is ours to maintain; anything else belongs to the user.
            (None | Some(None), Some(existing)) => {
                let identical = content_hash(&existing) == default_hash;
                record
                    .tools
                    .insert(relative.to_string(), identical.then_some(default_hash));
            }
            // Deleted by the user.
            (Some(_), None) => {}
            (Some(Some(written)), Some(existing)) => {
                let untouched = content_hash(&existing) == *written;
                if untouched && *written != default_hash {
                    fs::write(&path, content)?;
                    record.tools.insert(relative.to_string(), Some(default_hash));
                    updated += 1;
                }
            }
        }
    }

    if added + updated > 0 {
        log::info!("default tools: {added} added, {updated} updated");
    }
    fs::write(record_path, serde_json::to_vec_pretty(&record).unwrap_or_default())
}

fn load_record(path: &Path) -> SeedRecord {
    let Ok(bytes) = fs::read(path) else {
        return SeedRecord::default();
    };
    if let Ok(record) = serde_json::from_slice::<SeedRecord>(&bytes) {
        return record;
    }
    // Before hashes were recorded the file was a plain list of paths. Their state is unknown:
    // files identical to the current default are adopted, all others count as user-owned.
    let legacy: Vec<String> = serde_json::from_slice(&bytes).unwrap_or_default();
    SeedRecord {
        tools: legacy.into_iter().map(|path| (path, None)).collect(),
    }
}

/// FNV-1a over the content with normalized line endings (editors may convert them).
fn content_hash(content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in content.bytes().filter(|b| *b != b'\r') {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn write_new(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sandbox(PathBuf);

    impl Sandbox {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("cephi-seed-test-{}", uuid::Uuid::new_v4())))
        }
        fn sync(&self, defaults: &[(&str, &str)]) {
            sync_default_tools(&self.0.join("record.json"), &self.0.join("tools"), defaults).unwrap();
        }
        fn read(&self, name: &str) -> Option<String> {
            fs::read_to_string(self.0.join("tools").join(name)).ok()
        }
        fn write(&self, name: &str, content: &str) {
            write_new(&self.0.join("tools").join(name), content).unwrap();
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn untouched_defaults_follow_updates_edited_ones_stay() {
        let sandbox = Sandbox::new();
        sandbox.sync(&[("a.json", "v1"), ("b.json", "v1"), ("c.json", "v1")]);
        assert_eq!(sandbox.read("a.json").as_deref(), Some("v1"));

        sandbox.write("b.json", "mine");
        fs::remove_file(sandbox.0.join("tools/c.json")).unwrap();
        sandbox.sync(&[("a.json", "v2"), ("b.json", "v2"), ("c.json", "v2"), ("d.json", "new")]);

        assert_eq!(
            sandbox.read("a.json").as_deref(),
            Some("v2"),
            "untouched default is updated"
        );
        assert_eq!(
            sandbox.read("b.json").as_deref(),
            Some("mine"),
            "edited default is kept"
        );
        assert_eq!(sandbox.read("c.json"), None, "deleted default stays deleted");
        assert_eq!(sandbox.read("d.json").as_deref(), Some("new"), "new default is added");
    }

    #[test]
    fn legacy_record_adopts_files_equal_to_the_default() {
        let sandbox = Sandbox::new();
        sandbox.write("a.json", "v1");
        sandbox.write("b.json", "old");
        fs::write(sandbox.0.join("record.json"), r#"["a.json", "b.json"]"#).unwrap();

        sandbox.sync(&[("a.json", "v1"), ("b.json", "v1")]);
        sandbox.sync(&[("a.json", "v2"), ("b.json", "v2")]);
        assert_eq!(sandbox.read("a.json").as_deref(), Some("v2"));
        assert_eq!(sandbox.read("b.json").as_deref(), Some("old"));
    }

    #[test]
    fn hash_ignores_line_endings() {
        assert_eq!(content_hash("a\r\nb"), content_hash("a\nb"));
        assert_ne!(content_hash("a"), content_hash("b"));
    }
}
