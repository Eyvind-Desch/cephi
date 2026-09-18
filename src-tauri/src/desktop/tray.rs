use rust_i18n::t;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use super::window;
use crate::error::{AppError, AppResult};

const TRAY_ID: &str = "cephi";
const SHOW: &str = "show";
const OPEN_CONFIG: &str = "open-config";
const RELOAD: &str = "reload";
const QUIT: &str = "quit";

fn build_menu(app: &AppHandle) -> AppResult<Menu<Wry>> {
    let item = |id: &str, label: std::borrow::Cow<'_, str>| MenuItem::with_id(app, id, label, true, None::<&str>);
    Ok(Menu::with_items(
        app,
        &[
            &item(SHOW, t!("tray.show"))?,
            &PredefinedMenuItem::separator(app)?,
            &item(OPEN_CONFIG, t!("tray.open_config"))?,
            &item(RELOAD, t!("tray.reload"))?,
            &PredefinedMenuItem::separator(app)?,
            &item(QUIT, t!("tray.quit"))?,
        ],
    )?)
}

/// Rebuilds the menu texts, e.g. after the language changed.
pub fn refresh_menu(app: &AppHandle) -> AppResult<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_menu(app)?))?;
    }
    Ok(())
}

pub fn create(app: &AppHandle) -> AppResult<()> {
    let menu = build_menu(app)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| AppError::msg("app icon missing"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Cephi")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let result = match event.id.as_ref() {
                SHOW => window::show(app),
                OPEN_CONFIG => crate::app::open_config_folder(app),
                RELOAD => {
                    crate::app::reload_config(app);
                    Ok(())
                }
                QUIT => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(e) = result {
                log::error!("tray action {}: {e}", event.id.as_ref());
            }
        })
        .on_tray_icon_event(|tray, event| {
            // Always show (not toggle): clicking the tray already takes focus away from the window.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(e) = window::show(tray.app_handle()) {
                    log::error!("show window: {e}");
                }
            }
        })
        .build(app)?;

    Ok(())
}
