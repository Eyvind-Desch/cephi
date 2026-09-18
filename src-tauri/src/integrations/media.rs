//! Media control: playback via system media keys (works with any player), volume via Core Audio.

use rust_i18n::t;
use serde::Deserialize;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK,
};

use super::audio::{self, VolumeState};

const DEFAULT_VOLUME_STEP: i32 = 10;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaAction {
    PlayPause,
    Next,
    Previous,
    SetVolume,
    VolumeUp,
    VolumeDown,
    Mute,
    Unmute,
}

impl MediaAction {
    pub fn parse(value: &str) -> Result<Self, String> {
        serde_json::from_value(serde_json::Value::String(value.to_string()))
            .map_err(|_| t!("media.unknown_action", action = value).to_string())
    }

    /// `level`: target volume for `set_volume`; `amount`: step for `volume_up`/`volume_down`.
    /// Volume actions also return the new volume (for the level bar).
    pub fn run(self, level: Option<i64>, amount: Option<i64>) -> Result<(String, Option<VolumeState>), String> {
        let step = amount.map_or(DEFAULT_VOLUME_STEP, |a| a.clamp(1, 100) as i32);
        let volume = match self {
            MediaAction::PlayPause => return Ok((press(VK_MEDIA_PLAY_PAUSE, t!("media.play_pause")), None)),
            MediaAction::Next => return Ok((press(VK_MEDIA_NEXT_TRACK, t!("media.next")), None)),
            MediaAction::Previous => return Ok((press(VK_MEDIA_PREV_TRACK, t!("media.previous")), None)),
            MediaAction::SetVolume => {
                let level = level.ok_or_else(|| t!("media.level_missing").to_string())?;
                if !(0..=100).contains(&level) {
                    return Err(t!("media.level_out_of_range", level = level).into());
                }
                audio::set(level as i32)?
            }
            MediaAction::VolumeUp => audio::change(step)?,
            MediaAction::VolumeDown => audio::change(-step)?,
            MediaAction::Mute => audio::set_muted(None)?,
            MediaAction::Unmute => audio::set_muted(Some(false))?,
        };
        Ok((audio::describe(volume), Some(volume)))
    }
}

fn press(vk: u16, message: impl Into<String>) -> String {
    // SAFETY: plain synthetic key events; no pointers involved.
    unsafe {
        keybd_event(vk as u8, 0, KEYEVENTF_EXTENDEDKEY, 0);
        keybd_event(vk as u8, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
    }
    message.into()
}
