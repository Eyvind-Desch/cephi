//! What the user can attach to a message from the machine itself: a screenshot and dictation.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::desktop::{capture, window};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// How long the recording thread gets to drain its buffer after a stop.
const RECORDER_DRAIN: Duration = Duration::from_millis(60);

/// Time for the window to disappear before the picture is taken.
const HIDE_BEFORE_CAPTURE: Duration = Duration::from_millis(180);

/// Stops the recording and turns it into text. An empty result means nothing was understood.
pub async fn dictate(state: &AppState) -> AppResult<String> {
    state.recorder.request_stop();
    // Give the recording thread a moment to hand over what is still in the audio buffer.
    tokio::time::sleep(RECORDER_DRAIN).await;
    let wav = state.recorder.take().map_err(AppError::msg)?;
    let config = state.config();
    crate::voice::transcribe(&state.http, &config.settings.voice, wav)
        .await
        .map_err(AppError::msg)
}

/// A screenshot of the monitor the mouse is on. Cephi hides itself first, so the picture shows
/// what is behind it, and comes back afterwards.
pub async fn screenshot(app: &AppHandle) -> AppResult<capture::ScreenshotImage> {
    let window = app.get_window(window::MAIN_LABEL);
    let was_visible = window.as_ref().is_some_and(|w| w.is_visible().unwrap_or(false));
    if was_visible {
        if let Some(window) = &window {
            let _ = window.hide();
        }
        tokio::time::sleep(HIDE_BEFORE_CAPTURE).await;
    }

    // GDI and COM: off the async runtime threads.
    let shot = tauri::async_runtime::spawn_blocking(capture::monitor_at_cursor)
        .await
        .map_err(|e| AppError::msg(format!("internal error: {e}")))?;

    if was_visible {
        if let Err(e) = window::show(app) {
            log::warn!("show after screenshot: {e}");
        }
    }
    shot.map(Into::into).map_err(AppError::msg)
}
