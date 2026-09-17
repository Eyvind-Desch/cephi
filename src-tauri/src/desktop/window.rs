use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Window};

use crate::config::{Anchor, WindowSettings};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const MAIN_LABEL: &str = "main";
pub const EVENT_SHOWN: &str = "orbit://shown";

/// Focus changes inside WebView2 can briefly report the window as unfocused; wait before hiding.
const BLUR_HIDE_DELAY: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    #[default]
    Compact,
    Expanded,
    /// Wider panel for the web tab.
    Web,
}

#[derive(Default)]
pub struct WindowController {
    pinned: AtomicBool,
    mode: Mutex<WindowMode>,
}

impl WindowController {
    pub fn is_pinned(&self) -> bool {
        self.pinned.load(Ordering::Relaxed)
    }

    pub fn set_pinned(&self, pinned: bool) {
        self.pinned.store(pinned, Ordering::Relaxed);
    }

    fn mode(&self) -> WindowMode {
        *self.mode.lock().expect("mode lock poisoned")
    }
}

/// The window as such, not `WebviewWindow`: with the web tab it hosts more than one webview.
fn main_window(app: &AppHandle) -> AppResult<Window> {
    app.get_window(MAIN_LABEL)
        .ok_or_else(|| AppError::msg("main window not found"))
}

pub fn show(app: &AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    let state = app.state::<AppState>();
    layout(app, &window, state.window.mode(), &state.config().settings.window)?;
    window.show()?;
    window.set_focus()?;
    app.emit(EVENT_SHOWN, ())?;
    Ok(())
}

pub fn hide(app: &AppHandle) -> AppResult<()> {
    main_window(app)?.hide()?;
    Ok(())
}

/// Hotkey behaviour: hide when in front, otherwise bring to front.
pub fn toggle(app: &AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    if window.is_visible()? && window.is_focused()? {
        hide(app)
    } else {
        show(app)
    }
}

pub fn set_mode(app: &AppHandle, mode: WindowMode) -> AppResult<()> {
    let state = app.state::<AppState>();
    *state.window.mode.lock().expect("mode lock poisoned") = mode;
    let window = main_window(app)?;
    if window.is_visible()? {
        layout(app, &window, mode, &state.config().settings.window)?;
    }
    Ok(())
}

pub fn on_focus_lost(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(BLUR_HIDE_DELAY).await;
        let state = app.state::<AppState>();
        if state.window.is_pinned() || !state.config().settings.window.hide_on_blur {
            return;
        }
        if let Ok(window) = main_window(&app) {
            if window.is_visible().unwrap_or(false) && !window.is_focused().unwrap_or(true) {
                let _ = window.hide();
            }
        }
    });
}

/// Sizes the window for `mode` and anchors it inside the work area (excluding the taskbar)
/// of the monitor under the mouse cursor.
fn layout(app: &AppHandle, window: &Window, mode: WindowMode, settings: &WindowSettings) -> AppResult<()> {
    let size = match mode {
        WindowMode::Compact => settings.compact,
        WindowMode::Expanded => settings.expanded,
        WindowMode::Web => settings.web,
    };

    let Some(monitor) = target_monitor(app, window) else {
        window.set_size(tauri::LogicalSize::new(size.width, size.height))?;
        return Ok(());
    };

    let scale = monitor.scale_factor();
    let px = |logical: u32| (f64::from(logical) * scale).round() as i32;
    let margin = px(settings.margin);
    let area = monitor.work_area();
    let (left, top) = (area.position.x, area.position.y);
    let (area_width, area_height) = (area.size.width as i32, area.size.height as i32);
    // Large sizes (the web tab) must still fit on small screens.
    let width = px(size.width).min(area_width - 2 * margin);
    let height = px(size.height).min(area_height - 2 * margin);

    let x = match settings.anchor {
        Anchor::BottomLeft | Anchor::TopLeft => left + margin,
        Anchor::BottomCenter | Anchor::TopCenter => left + (area_width - width) / 2,
        Anchor::BottomRight | Anchor::TopRight => left + area_width - width - margin,
    };
    let y = match settings.anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => top + margin,
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => top + area_height - height - margin,
    };

    // Position first: moving to a monitor with a different DPI rescales the window.
    window.set_position(PhysicalPosition::new(x, y))?;
    window.set_size(PhysicalSize::new(width as u32, height as u32))?;
    Ok(())
}

fn target_monitor(app: &AppHandle, window: &Window) -> Option<Monitor> {
    app.cursor_position()
        .ok()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten())
}
