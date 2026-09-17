use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use super::defaults;
use crate::error::{AppError, AppResult};

/// Environment variable to override the config folder (useful for development).
const CONFIG_DIR_ENV: &str = "ORBIT_CONFIG_DIR";

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
                .map_err(|e| AppError::msg(format!("Konfigurationsordner nicht gefunden: {e}")))?
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

    /// Creates the folder structure and seeds defaults on first run.
    ///
    /// - Schemas are always rewritten so they match the running version.
    /// - `settings.json` is created only if missing.
    /// - Default tools are seeded only if the `tools/` folder does not exist yet, so tools
    ///   the user deleted stay deleted.
    pub fn ensure_initialized(&self) -> io::Result<()> {
        fs::create_dir_all(&self.schemas)?;
        for (name, content) in defaults::SCHEMAS {
            fs::write(self.schemas.join(name), content)?;
        }

        if !self.settings.exists() {
            fs::write(&self.settings, defaults::SETTINGS)?;
            log::info!("seeded {}", self.settings.display());
        }

        if !self.tools.exists() {
            for (relative, content) in defaults::TOOLS {
                write_new(&self.tools.join(relative), content)?;
            }
            log::info!("seeded {} default tool(s)", defaults::TOOLS.len());
        }

        Ok(())
    }
}

fn write_new(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}
