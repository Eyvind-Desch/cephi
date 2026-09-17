use rust_i18n::t;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::window;

/// (Re-)registers the global hotkey that toggles the main window.
pub fn register(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| t!("hotkey.reset_failed", error = e).to_string())?;

    shortcuts
        .on_shortcut(hotkey, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                if let Err(e) = window::toggle(app) {
                    log::error!("toggle window: {e}");
                }
            }
        })
        .map_err(|e| t!("hotkey.register_failed", hotkey = hotkey, error = e).to_string())?;

    log::info!("hotkey registered: {hotkey}");
    Ok(())
}
