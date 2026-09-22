//! Picking a region out of the screen, the way the Snipping Tool does it.
//!
//! The monitor is photographed **first** and the picture is then shown, frozen, in a window that
//! covers exactly that monitor. Everything after that happens on a still image: the overlay can
//! never end up in the shot, nothing moves under the selection, and the cut-out is taken from the
//! picture in the webview (a canvas), so no second grab is needed.

use std::sync::Mutex;

use rust_i18n::t;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::capture::{MonitorArea, Screenshot, ScreenshotImage};
use super::window;
use crate::error::{AppError, AppResult};

/// Label of the overlay window and its webview.
pub const OVERLAY_LABEL: &str = "shot";
/// How long the overlay may take to put the picture on screen before it is shown regardless.
const SHOW_ANYWAY: std::time::Duration = std::time::Duration::from_millis(1500);
/// The picked region reaches the main window through this.
pub const EVENT_REGION: &str = "cephi://screenshot-region";

/// The frozen picture, waiting for the overlay to ask for it.
#[derive(Default)]
pub struct Pending(Mutex<Option<ScreenshotImage>>);

impl Pending {
    fn put(&self, image: ScreenshotImage) {
        *self.0.lock().expect("pending shot lock poisoned") = Some(image);
    }

    fn take(&self) -> Option<ScreenshotImage> {
        self.0.lock().expect("pending shot lock poisoned").take()
    }

    fn clear(&self) {
        let _ = self.take();
    }
}

/// Opens the overlay over `area`, showing `shot`. It stays hidden until the picture is on screen,
/// so nobody sees an empty window while the page loads.
pub fn open(app: &AppHandle, shot: Screenshot, area: MonitorArea) -> AppResult<()> {
    close(app);
    app.state::<Pending>().put(shot.into());

    let window = WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App(format!("index.html?window={OVERLAY_LABEL}").into()))
        .title("Cephi")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .visible(false)
        // A size is required up front; the real one is set below, in physical pixels.
        .inner_size(800.0, 600.0)
        .build()
        .map_err(|e| AppError::msg(t!("capture.failed", error = e)))?;

    let place = |window: &tauri::WebviewWindow| -> tauri::Result<()> {
        window.set_position(PhysicalPosition::new(area.left, area.top))?;
        window.set_size(PhysicalSize::new(area.width, area.height))
    };
    // Twice: moving to a monitor with another scaling rescales the window, the second call corrects it.
    place(&window)
        .and_then(|()| place(&window))
        .map_err(|e| AppError::msg(t!("capture.failed", error = e)))?;

    // Safety net: if the page never reports back (a failed load), the overlay must not stay
    // invisible with Cephi hidden behind it - Escape only works once something is on screen.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SHOW_ANYWAY).await;
        if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
            if !window.is_visible().unwrap_or(true) {
                log::warn!("region overlay did not report back; showing it anyway");
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });
    Ok(())
}

/// The frozen picture for the overlay.
pub fn pending(app: &AppHandle) -> Option<ScreenshotImage> {
    let pending = app.state::<Pending>();
    let image = pending.take();
    // Put it back: a reload of the overlay must find it again.
    if let Some(image) = &image {
        pending.put(image.clone());
    }
    image
}

/// The overlay has the picture on screen and can be shown.
pub fn ready(app: &AppHandle) -> AppResult<()> {
    let Some(window) = app.get_webview_window(OVERLAY_LABEL) else {
        return Ok(());
    };
    window.show()?;
    window.set_focus()?;
    // Escape and Enter only arrive when Windows actually hands over the foreground.
    if let Some(window) = app.get_window(OVERLAY_LABEL) {
        if !window::is_foreground(&window) {
            window::force_foreground(&window);
        }
    }
    Ok(())
}

/// The user picked a region: the overlay goes away, Cephi comes back with the cut-out.
pub fn finish(app: &AppHandle, image: ScreenshotImage) -> AppResult<()> {
    close(app);
    window::show(app)?;
    app.emit_to(window::MAIN_LABEL, EVENT_REGION, image)?;
    Ok(())
}

/// Escape, or a click without dragging: nothing is kept, Cephi comes back.
pub fn cancel(app: &AppHandle) -> AppResult<()> {
    close(app);
    window::show(app)
}

fn close(app: &AppHandle) {
    app.state::<Pending>().clear();
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.close();
    }
}
