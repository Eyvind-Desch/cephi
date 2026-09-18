//! Reads the text selected in the foreground app.
//!
//! Windows has no reliable way to *read* a selection across all apps (UI Automation only works where
//! the focused element exposes text, which browsers often don't). So Orbit does what comparable tools
//! do: it sends Ctrl+C to the foreground app and **restores the previous clipboard afterwards**.
//! This only happens on the separate selection hotkey, never on the normal one.

use std::time::{Duration, Instant};

use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYEVENTF_KEYUP, VK_C, VK_CONTROL, VK_LWIN, VK_MENU, VK_SHIFT,
};

/// How long the app may take to put the selection on the clipboard.
const COPY_TIMEOUT: Duration = Duration::from_millis(500);
const POLL_INTERVAL: Duration = Duration::from_millis(20);
/// Give the source app a moment to finish its copy before restoring the clipboard.
const RESTORE_DELAY: Duration = Duration::from_millis(120);

/// Returns the selected text, or `None` if nothing was selected.
pub async fn capture() -> Option<String> {
    tauri::async_runtime::spawn_blocking(copy_selection).await.ok().flatten()
}

fn copy_selection() -> Option<String> {
    let previous = clipboard_text();
    // SAFETY: plain query, no arguments.
    let before = unsafe { GetClipboardSequenceNumber() };

    send_copy();

    let deadline = Instant::now() + COPY_TIMEOUT;
    while Instant::now() < deadline {
        std::thread::sleep(POLL_INTERVAL);
        // SAFETY: see above.
        if unsafe { GetClipboardSequenceNumber() } != before {
            std::thread::sleep(RESTORE_DELAY);
            let copied = clipboard_text();
            restore(previous);
            return copied.filter(|text| !text.trim().is_empty());
        }
    }
    log::info!("selection hotkey: nothing was copied");
    None
}

/// Presses Ctrl+C. The modifiers of the hotkey itself are still held down, so they are released
/// first — otherwise the app would see Ctrl+Alt+C.
fn send_copy() {
    // SAFETY: synthetic key events only, no pointers involved.
    unsafe {
        for modifier in [VK_MENU, VK_SHIFT, VK_LWIN] {
            keybd_event(modifier as u8, 0, KEYEVENTF_KEYUP, 0);
        }
        keybd_event(VK_CONTROL as u8, 0, 0, 0);
        keybd_event(VK_C as u8, 0, 0, 0);
        keybd_event(VK_C as u8, 0, KEYEVENTF_KEYUP, 0);
        keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Puts back what was on the clipboard before. Non-text content (e.g. an image) cannot be restored;
/// then the copied text stays, which is the same as a manual copy.
fn restore(previous: Option<String>) {
    let Some(text) = previous else { return };
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Err(e) = clipboard.set_text(text) {
            log::warn!("could not restore the clipboard: {e}");
        }
    }
}
