use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use super::window;
use crate::error::{AppError, AppResult};

const SHOW: &str = "show";
const OPEN_CONFIG: &str = "open-config";
const RELOAD: &str = "reload";
const QUIT: &str = "quit";

pub fn create(app: &AppHandle) -> AppResult<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, SHOW, "Orbit öffnen", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, OPEN_CONFIG, "Konfiguration öffnen", true, None::<&str>)?,
            &MenuItem::with_id(app, RELOAD, "Konfiguration neu laden", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT, "Beenden", true, None::<&str>)?,
        ],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| AppError::msg("App-Icon fehlt."))?;

    TrayIconBuilder::with_id("orbit")
        .icon(icon)
        .tooltip("Orbit")
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
