//! System volume of the default output device via Windows Core Audio.
//!
//! Unlike media keys (fixed 2 % steps), this sets exact levels: "Lautstärke auf 30 %", "24 lauter".

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

/// Volume in percent (0–100) and mute state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolumeState {
    pub percent: u8,
    pub muted: bool,
}

pub fn get() -> Result<VolumeState, String> {
    let endpoint = endpoint()?;
    read(&endpoint)
}

pub fn set(percent: i32) -> Result<VolumeState, String> {
    let endpoint = endpoint()?;
    let level = percent.clamp(0, 100) as f32 / 100.0;
    // SAFETY: valid COM interface; a null event context is allowed.
    unsafe {
        endpoint
            .SetMasterVolumeLevelScalar(level, std::ptr::null())
            .map_err(|e| format!("Lautstärke konnte nicht gesetzt werden: {e}"))?;
        // Changing the volume should be audible.
        let _ = endpoint.SetMute(false, std::ptr::null());
    }
    read(&endpoint)
}

pub fn change(delta: i32) -> Result<VolumeState, String> {
    let current = get()?;
    set(i32::from(current.percent) + delta)
}

/// `None` toggles.
pub fn set_muted(muted: Option<bool>) -> Result<VolumeState, String> {
    let endpoint = endpoint()?;
    let muted = muted.unwrap_or(!read(&endpoint)?.muted);
    // SAFETY: valid COM interface; a null event context is allowed.
    unsafe {
        endpoint
            .SetMute(muted, std::ptr::null())
            .map_err(|e| format!("Stummschaltung fehlgeschlagen: {e}"))?;
    }
    read(&endpoint)
}

fn read(endpoint: &IAudioEndpointVolume) -> Result<VolumeState, String> {
    // SAFETY: valid COM interface.
    unsafe {
        let level = endpoint
            .GetMasterVolumeLevelScalar()
            .map_err(|e| format!("Lautstärke nicht lesbar: {e}"))?;
        let muted = endpoint.GetMute().map(|m| m.as_bool()).unwrap_or(false);
        Ok(VolumeState {
            percent: (level * 100.0).round() as u8,
            muted,
        })
    }
}

fn endpoint() -> Result<IAudioEndpointVolume, String> {
    // SAFETY: standard COM activation. CoInitializeEx may report that COM is already initialized
    // on this thread (S_FALSE / RPC_E_CHANGED_MODE); both are fine for these calls.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("Audiogeräte nicht verfügbar: {e}"))?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|_| "Kein Audio-Ausgabegerät gefunden.".to_string())?;
        device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| format!("Lautstärkeregelung nicht verfügbar: {e}"))
    }
}

pub fn describe(state: VolumeState) -> String {
    if state.muted {
        format!("Lautstärke {} % (stumm)", state.percent)
    } else {
        format!("Lautstärke {} %", state.percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Briefly changes the real system volume and restores it. Run manually:
    /// `cargo test -- --ignored set_and_restore_volume`
    #[test]
    #[ignore = "changes the real system volume"]
    fn set_and_restore_volume() {
        let Ok(original) = get() else { return };
        let target = if original.percent > 50 { 40 } else { 60 };
        assert_eq!(set(target).unwrap().percent, target as u8);
        assert_eq!(change(-5).unwrap().percent, target as u8 - 5);
        set(i32::from(original.percent)).unwrap();
        set_muted(Some(original.muted)).unwrap();
        assert_eq!(get().unwrap(), original);
    }
}
