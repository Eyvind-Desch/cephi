//! System media keys: control whatever player is active (Spotify, browser, …) without any API.

use serde::Deserialize;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
    VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};

/// One volume key press changes the volume by 2 %.
const VOLUME_STEPS: usize = 5;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    PlayPause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    Mute,
}

impl MediaAction {
    pub fn parse(value: &str) -> Result<Self, String> {
        serde_json::from_value(serde_json::Value::String(value.to_string()))
            .map_err(|_| format!("Unbekannte Medienaktion \"{value}\"."))
    }

    pub fn run(self) -> String {
        let (key, presses, message) = match self {
            MediaAction::PlayPause => (VK_MEDIA_PLAY_PAUSE, 1, "Wiedergabe umgeschaltet."),
            MediaAction::Next => (VK_MEDIA_NEXT_TRACK, 1, "Nächster Titel."),
            MediaAction::Previous => (VK_MEDIA_PREV_TRACK, 1, "Vorheriger Titel."),
            MediaAction::VolumeUp => (VK_VOLUME_UP, VOLUME_STEPS, "Lauter (+10 %)."),
            MediaAction::VolumeDown => (VK_VOLUME_DOWN, VOLUME_STEPS, "Leiser (−10 %)."),
            MediaAction::Mute => (VK_VOLUME_MUTE, 1, "Stummschaltung umgeschaltet."),
        };
        for _ in 0..presses {
            press(key as u8);
        }
        message.to_string()
    }
}

fn press(vk: u8) {
    // SAFETY: plain synthetic key events; no pointers involved.
    unsafe {
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY, 0);
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
    }
}
