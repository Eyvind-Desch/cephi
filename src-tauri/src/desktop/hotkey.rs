use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::window;

/// (Re-)registers the global hotkey that toggles the main window.
pub fn register(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts
        .unregister_all()
        .map_err(|e| format!("Hotkeys konnten nicht zurückgesetzt werden: {e}"))?;

    shortcuts
        .on_shortcut(hotkey, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                if let Err(e) = window::toggle(app) {
                    log::error!("toggle window: {e}");
                }
            }
        })
        .map_err(|e| {
            format!("Hotkey \"{hotkey}\" konnte nicht registriert werden ({e}). Evtl. von einer anderen App belegt?")
        })?;

    log::info!("hotkey registered: {hotkey}");
    Ok(())
}
