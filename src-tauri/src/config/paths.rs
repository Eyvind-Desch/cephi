use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use super::defaults;
use crate::error::{AppError, AppResult};

/// Environment variable to override the config folder (useful for development).
const CONFIG_DIR_ENV: &str = "ORBIT_CONFIG_DIR";
/// Remembers which default tools were already offered, so deleted ones are not re-created.
const SEEDED_TOOLS_FILE: &str = ".seeded-tools.json";

/// Locations of all user-editable configuration files.
///
/// Default root: `%APPDATA%\Orbit`
/// ```text
/// Orbit/
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
}

impl ConfigPaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let root = match std::env::var_os(CONFIG_DIR_ENV) {
            Some(dir) => PathBuf::from(dir),
            None => app
                .path()
                .config_dir()
                .map_err(|e| AppError::msg(format!("config directory not found: {e}")))?
                .join("Orbit"),
        };
        Ok(Self::from_root(root))
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self {
            settings: root.join("settings.json"),
            tools: root.join("tools"),
            schemas: root.join("schemas"),
            root,
        }
    }

    /// Creates the folder structure and seeds defaults.
    ///
    /// - Schemas are always rewritten so they match the running version.
    /// - `settings.json` is created only if missing.
    /// - Each default tool is seeded once: new defaults from app updates appear automatically,
    ///   but tools the user deleted or edited are never touched again.
    pub fn ensure_initialized(&self) -> io::Result<()> {
        fs::create_dir_all(&self.schemas)?;
        for (name, content) in defaults::SCHEMAS {
            fs::write(self.schemas.join(name), content)?;
        }

        if !self.settings.exists() {
            fs::write(&self.settings, defaults::SETTINGS)?;
            log::info!("seeded {}", self.settings.display());
        }

        self.seed_default_tools()
    }

    fn seed_default_tools(&self) -> io::Result<()> {
        let record = self.root.join(SEEDED_TOOLS_FILE);
        let mut seeded: Vec<String> = fs::read(&record)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();

        let mut added = 0;
        for (relative, content) in defaults::TOOLS {
            if seeded.iter().any(|s| s == relative) {
                continue;
            }
            let path = self.tools.join(relative);
            if !path.exists() {
                write_new(&path, content)?;
                added += 1;
            }
            seeded.push(relative.to_string());
        }

        if added > 0 {
            log::info!("seeded {added} default tool(s)");
        }
        fs::write(record, serde_json::to_vec_pretty(&seeded).unwrap_or_default())
    }
}

fn write_new(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}
