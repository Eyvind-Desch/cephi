use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager, Monitor, Window};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER,
};

use crate::config::{Anchor, WindowSettings};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const MAIN_LABEL: &str = "main";
pub const EVENT_SHOWN: &str = "cephi://shown";

/// Focus changes inside WebView2 can briefly report the window as unfocused; wait before hiding.
const BLUR_HIDE_DELAY: Duration = Duration::from_millis(150);
/// Duration of the resize animation between modes.
const RESIZE_DURATION: Duration = Duration::from_millis(180);
const FRAME: Duration = Duration::from_millis(8);

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    #[default]
    Compact,
    Expanded,
    /// The enlarged panel (e.g. for the web tab).
    Large,
}

#[derive(Default)]
pub struct WindowController {
    pinned: AtomicBool,
    mode: Mutex<WindowMode>,
    /// Incremented per resize; a running animation stops when it is outdated.
    animation: AtomicU64,
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

/// Window rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/// The window as such, not `WebviewWindow`: with the web tab it hosts more than one webview.
fn main_window(app: &AppHandle) -> AppResult<Window> {
    app.get_window(MAIN_LABEL)
        .ok_or_else(|| AppError::msg("main window not found"))
}

pub fn show(app: &AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    let state = app.state::<AppState>();
    let settings = &state.config().settings.window;
    // Appears on the monitor with the mouse cursor.
    let monitor = cursor_monitor(app).or_else(|| window.current_monitor().ok().flatten());
    if let Some(rect) = monitor.map(|m| target_rect(&m, state.window.mode(), settings)) {
        state.window.animation.fetch_add(1, Ordering::Relaxed);
        place(&window, rect)?;
        // Moving to a monitor with another DPI rescales the window; the second call corrects the size.
        place(&window, rect)?;
    }
    window.show()?;
    window.set_focus()?;
    // Keyboard focus must reach the webview itself, otherwise the input field cannot receive it.
    if let Some(webview) = app.get_webview(MAIN_LABEL) {
        webview.set_focus()?;
    }
    app.emit_to(MAIN_LABEL, EVENT_SHOWN, ())?;
    Ok(())
}

pub fn hide(app: &AppHandle) -> AppResult<()> {
    main_window(app)?.hide()?;
    Ok(())
}

/// Hotkey behaviour: hide when in front, otherwise bring to front.
pub fn toggle(app: &AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    if window.is_visible()? && is_foreground(&window) {
        hide(app)
    } else {
        show(app)
    }
}

/// Switches the size; animated while the window is visible.
pub fn set_mode(app: &AppHandle, mode: WindowMode) -> AppResult<()> {
    let state = app.state::<AppState>();
    *state.window.mode.lock().expect("mode lock poisoned") = mode;
    let window = main_window(app)?;
    if !window.is_visible()? {
        return Ok(());
    }
    let Some(monitor) = window.current_monitor()? else {
        return Ok(());
    };
    let target = target_rect(&monitor, mode, &state.config().settings.window);
    let generation = state.window.animation.fetch_add(1, Ordering::Relaxed) + 1;
    let from = current_rect(&window)?;

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        loop {
            let state = app.state::<AppState>();
            if state.window.animation.load(Ordering::Relaxed) != generation {
                return;
            }
            let progress = (started.elapsed().as_secs_f64() / RESIZE_DURATION.as_secs_f64()).min(1.0);
            if let Err(e) = place(&window, interpolate(from, target, ease_out_cubic(progress))) {
                log::warn!("resize animation: {e}");
                return;
            }
            if progress >= 1.0 {
                return;
            }
            tokio::time::sleep(FRAME).await;
        }
    });
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
            if window.is_visible().unwrap_or(false) && !is_foreground(&window) {
                log::info!("hidden: another window became active");
                let _ = window.hide();
            }
        }
    });
}

/// Size for `mode`, anchored inside the monitor's work area (excluding the taskbar).
fn target_rect(monitor: &Monitor, mode: WindowMode, settings: &WindowSettings) -> Rect {
    let size = match mode {
        WindowMode::Compact => settings.compact,
        WindowMode::Expanded => settings.expanded,
        WindowMode::Large => settings.large,
    };

    let scale = monitor.scale_factor();
    let px = |logical: u32| (f64::from(logical) * scale).round() as i32;
    let margin = px(settings.margin);
    let area = monitor.work_area();
    let (left, top) = (area.position.x, area.position.y);
    let (area_width, area_height) = (area.size.width as i32, area.size.height as i32);
    // Large sizes must still fit on small screens.
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
    Rect { x, y, width, height }
}

fn current_rect(window: &Window) -> AppResult<Rect> {
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    Ok(Rect {
        x: position.x,
        y: position.y,
        width: size.width as i32,
        height: size.height as i32,
    })
}

/// Whether Cephi is the active app. Unlike `is_focused`, this stays true while the keyboard focus is in a
/// child webview (the web tab): focus moving *inside* the window must not count as clicking elsewhere.
fn is_foreground(window: &Window) -> bool {
    // SAFETY: plain query without arguments.
    let foreground = unsafe { GetForegroundWindow() };
    window.hwnd().is_ok_and(|hwnd| std::ptr::eq(foreground, hwnd.0))
}

/// Moves and resizes in one step, so the anchored edge does not jitter during animations.
fn place(window: &Window, rect: Rect) -> AppResult<()> {
    let hwnd = window.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
    let flags = SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE;
    // SAFETY: `hwnd` belongs to a live window owned by this process.
    let ok = unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            flags,
        )
    };
    if ok == 0 {
        return Err(AppError::msg(format!(
            "SetWindowPos failed: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

fn interpolate(from: Rect, to: Rect, t: f64) -> Rect {
    let mix = |a: i32, b: i32| a + ((f64::from(b - a)) * t).round() as i32;
    Rect {
        x: mix(from.x, to.x),
        y: mix(from.y, to.y),
        width: mix(from.width, to.width),
        height: mix(from.height, to.height),
    }
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

fn cursor_monitor(app: &AppHandle) -> Option<Monitor> {
    app.cursor_position()
        .ok()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_hits_both_ends() {
        let from = Rect {
            x: 100,
            y: 800,
            width: 640,
            height: 68,
        };
        let to = Rect {
            x: -60,
            y: 100,
            width: 800,
            height: 768,
        };
        assert_eq!(interpolate(from, to, ease_out_cubic(0.0)), from);
        assert_eq!(interpolate(from, to, ease_out_cubic(1.0)), to);
        // Anchored bottom-right: the right edge stays put along the way.
        let mid = interpolate(from, to, 0.5);
        assert_eq!(mid.x + mid.width, from.x + from.width);
    }
}
