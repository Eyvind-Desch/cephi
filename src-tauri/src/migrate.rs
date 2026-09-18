//! One-time move from the old name "Orbit" to "Cephi".
//!
//! Runs at startup before anything reads its files. Old data is **copied**, never deleted, so a failed
//! move can't lose anything; once the new folders exist this does nothing. API keys move separately
//! on first read (`secrets/store.rs`).

use std::fs;
use std::io;
use std::path::Path;

use tauri::{AppHandle, Manager};

use crate::desktop::autostart;

/// `%APPDATA%\Orbit`: settings, tools, schemas.
const OLD_CONFIG_DIR: &str = "Orbit";
/// Tauri's per-app data folder under the old identifier (saved chats).
const OLD_IDENTIFIER: &str = "io.github.eyvind-desch.orbit";
/// The default system prompt started with this; the new name belongs there.
const OLD_PROMPT_START: &str = "You are Orbit, a desktop assistant";
const NEW_PROMPT_START: &str = "You are Cephi, a desktop assistant";

/// Copies what the old version left behind. Problems are logged, never fatal.
pub fn from_orbit(app: &AppHandle, config_root: &Path) {
    let path = app.path();
    if let Ok(config_dir) = path.config_dir() {
        let old = config_dir.join(OLD_CONFIG_DIR);
        if old.is_dir() && !config_root.exists() {
            match copy_dir(&old, config_root) {
                Ok(()) => {
                    log::info!("moved settings and tools from {}", old.display());
                    rename_in_prompt(&config_root.join("settings.json"));
                }
                Err(e) => log::warn!("could not take over {}: {e}", old.display()),
            }
        }
    }

    if let Ok(data_dir) = path.app_data_dir() {
        let old_chats = data_dir.with_file_name(OLD_IDENTIFIER).join("chats");
        let new_chats = data_dir.join("chats");
        if old_chats.is_dir() && !new_chats.exists() {
            match copy_dir(&old_chats, &new_chats) {
                Ok(()) => log::info!("moved saved chats from {}", old_chats.display()),
                Err(e) => log::warn!("could not take over saved chats: {e}"),
            }
        }
    }

    autostart::remove_legacy_entry();
}

/// The shipped system prompt names the assistant; a prompt the user wrote themselves stays as is.
fn rename_in_prompt(settings: &Path) {
    let Ok(text) = fs::read_to_string(settings) else { return };
    if text.contains(OLD_PROMPT_START) {
        if let Err(e) = fs::write(settings, text.replace(OLD_PROMPT_START, NEW_PROMPT_START)) {
            log::warn!("could not update the system prompt: {e}");
        }
    }
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_nested_folders_and_renames_the_default_prompt() {
        let root = std::env::temp_dir().join(format!("cephi-migrate-test-{}", uuid::Uuid::new_v4()));
        let old = root.join("Orbit");
        fs::create_dir_all(old.join("tools/web")).unwrap();
        fs::write(old.join("tools/web/x.json"), "{}").unwrap();
        fs::write(
            old.join("settings.json"),
            r#"{ "agent": { "systemPrompt": "You are Orbit, a desktop assistant on Windows." } }"#,
        )
        .unwrap();

        let new = root.join("Cephi");
        copy_dir(&old, &new).unwrap();
        rename_in_prompt(&new.join("settings.json"));

        assert!(new.join("tools/web/x.json").exists());
        assert!(fs::read_to_string(new.join("settings.json"))
            .unwrap()
            .contains("You are Cephi"));
        assert!(old.join("settings.json").exists(), "the old folder is kept");
        let _ = fs::remove_dir_all(root);
    }
}
