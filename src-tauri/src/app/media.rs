//! What the user can attach to a message from the machine itself: a screenshot and dictation.

use std::time::Duration;

use rust_i18n::t;
use tauri::{AppHandle, Manager};

use crate::desktop::{capture, region, window};
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
    let text = crate::voice::transcribe(&state.http, &config.settings.voice, wav)
        .await
        .map_err(AppError::msg)?;
    log::info!("dictation: {} character(s)", text.len());
    // An empty answer used to reach the composer as "", where nothing happened at all.
    if text.is_empty() {
        return Err(AppError::msg(t!("voice.not_understood")));
    }
    Ok(text)
}

/// A screenshot of the monitor the mouse is on. Cephi hides itself first, so the picture shows
/// what is behind it, and comes back afterwards.
pub async fn screenshot(app: &AppHandle) -> AppResult<capture::ScreenshotImage> {
    let shot = hidden_capture(app, capture::monitor_at_cursor).await?;
    if let Err(e) = window::show(app) {
        log::warn!("show after screenshot: {e}");
    }
    shot.map(Into::into).map_err(AppError::msg)
}

/// A screenshot to pick a region out of: the monitor is photographed the same way, but instead of
/// coming back Cephi puts the frozen picture on screen and lets the user drag a rectangle over it
/// (see [`region`]). Cephi returns once a region is picked or the overlay is cancelled.
pub async fn screenshot_region(app: &AppHandle) -> AppResult<()> {
    let taken = hidden_capture(app, capture::monitor_at_cursor_unscaled).await?;
    match taken {
        Ok((shot, area)) => region::open(app, shot, area),
        Err(message) => {
            let _ = window::show(app);
            Err(AppError::msg(message))
        }
    }
}

/// Hides Cephi, runs `take` off the async runtime threads (GDI and COM need that) and leaves the
/// window hidden: what happens next differs between the two ways of taking a picture.
async fn hidden_capture<T: Send + 'static>(
    app: &AppHandle,
    take: fn() -> Result<T, String>,
) -> AppResult<Result<T, String>> {
    if app
        .get_window(window::MAIN_LABEL)
        .is_some_and(|w| w.is_visible().unwrap_or(false))
    {
        let _ = window::hide(app);
        tokio::time::sleep(HIDE_BEFORE_CAPTURE).await;
    }
    tauri::async_runtime::spawn_blocking(take)
        .await
        .map_err(|e| AppError::msg(format!("internal error: {e}")))
}
