use rust_i18n::t;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use super::window::{self, MAIN_LABEL};
use crate::config::Shortcut;
use crate::integrations::selection;

/// Selected text picked up by the selection hotkey (`None` = nothing was selected).
pub const EVENT_SELECTION: &str = "cephi://selection";

/// What a user-defined shortcut asks the UI to do.
pub const EVENT_ACTION: &str = "cephi://shortcut";

/// (Re-)registers the global hotkeys: toggling the window, picking up the selected text, and
/// whatever the user bound in `shortcuts`.
pub fn register(
    app: &AppHandle,
    hotkey: &str,
    selection_hotkey: Option<&str>,
    custom: &[Shortcut],
) -> Result<(), String> {
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

    register_custom(app, custom)?;

    let Some(selection_hotkey) = selection_hotkey.map(str::trim).filter(|h| !h.is_empty()) else {
        return Ok(());
    };
    shortcuts
        .on_shortcut(selection_hotkey, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                pick_up_selection(app.clone());
            }
        })
        .map_err(|e| t!("hotkey.register_failed", hotkey = selection_hotkey, error = e).to_string())?;
    log::info!("selection hotkey registered: {selection_hotkey}");
    Ok(())
}

/// Copies the selection *before* Cephi takes focus, then shows the window with it.
fn pick_up_selection(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let text = selection::capture().await;
        log::info!(
            "selection hotkey: {} characters",
            text.as_ref().map_or(0, |t| t.chars().count())
        );
        if let Err(e) = window::show(&app) {
            log::error!("show window: {e}");
        }
        if let Err(e) = app.emit_to(MAIN_LABEL, EVENT_SELECTION, text) {
            log::error!("emit selection: {e}");
        }
    });
}

/// The user's own shortcuts. One that Windows refuses (already taken by another app) is reported;
/// the ones before it stay registered, so a single bad entry does not cost all of them.
fn register_custom(app: &AppHandle, shortcuts: &[Shortcut]) -> Result<(), String> {
    for shortcut in shortcuts {
        let key = shortcut.key.trim();
        if key.is_empty() {
            continue;
        }
        let binding = shortcut.clone();
        app.global_shortcut()
            .on_shortcut(key, move |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    run(app, &binding);
                }
            })
            .map_err(|e| t!("hotkey.register_failed", hotkey = key, error = e).to_string())?;
        log::info!("shortcut registered: {key} → {:?}", shortcut.action);
    }
    Ok(())
}

/// Brings Cephi up and lets the UI carry the action out: everything a shortcut can do, the UI can
/// already do (open a view, take a screenshot, send a message), including the fast path for `prompt`.
fn run(app: &AppHandle, shortcut: &Shortcut) {
    if let Err(e) = window::show(app) {
        log::error!("show window: {e}");
    }
    if let Err(e) = app.emit_to(MAIN_LABEL, EVENT_ACTION, shortcut) {
        log::error!("emit shortcut: {e}");
    }
}
