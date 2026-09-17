//! Bluetooth and Wi-Fi radios via `Windows.Devices.Radios`.
//!
//! Every answer reflects the *actual* radio state read from Windows, so the model never has to
//! guess whether something is on.

use rust_i18n::t;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    Bluetooth,
    Wifi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    On,
    Off,
    Toggle,
    Status,
}

impl Device {
    /// Accepts English and German names ("wifi", "wlan", "wi-fi", "bluetooth", "bt").
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().replace(['-', ' '], "").as_str() {
            "bluetooth" | "bt" => Ok(Self::Bluetooth),
            "wlan" | "wifi" | "wireless" => Ok(Self::Wifi),
            other => Err(t!("radio.unknown_device", device = other).into()),
        }
    }

    fn kind(self) -> RadioKind {
        match self {
            Self::Bluetooth => RadioKind::Bluetooth,
            Self::Wifi => RadioKind::WiFi,
        }
    }

    fn label(self) -> String {
        match self {
            Self::Bluetooth => t!("radio.bluetooth").into(),
            Self::Wifi => t!("radio.wifi").into(),
        }
    }
}

impl Command {
    /// Accepts English and German words ("on"/"an"/"ein", "off"/"aus").
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "on" | "an" | "ein" => Ok(Self::On),
            "off" | "aus" => Ok(Self::Off),
            "toggle" | "umschalten" => Ok(Self::Toggle),
            "status" => Ok(Self::Status),
            other => Err(t!("radio.unknown_command", command = other).into()),
        }
    }
}

/// Blocking: call from a blocking thread.
pub fn run(device: Device, command: Command) -> Result<String, String> {
    let win = |e: windows::core::Error| t!("radio.windows_error", error = e.message()).to_string();

    // SAFETY: initializing COM for this thread; "already initialized" results are fine.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    if Radio::RequestAccessAsync().and_then(|op| op.get()).map_err(win)? != RadioAccessStatus::Allowed {
        return Err(t!("radio.access_denied").into());
    }

    let name = device.label();
    let radio = find(device)
        .map_err(win)?
        .ok_or_else(|| t!("radio.no_adapter", device = name).to_string())?;
    let current = radio.State().map_err(win)?;

    if current == RadioState::Disabled {
        return Err(t!("radio.hardware_disabled", device = name).into());
    }

    let target = match command {
        Command::Status => return Ok(t!("radio.status", device = name, state = describe(current)).into()),
        Command::On => RadioState::On,
        Command::Off => RadioState::Off,
        Command::Toggle if current == RadioState::On => RadioState::Off,
        Command::Toggle => RadioState::On,
    };

    if current == target {
        return Ok(t!("radio.already", device = name, state = describe(current)).into());
    }

    let status = radio.SetStateAsync(target).and_then(|op| op.get()).map_err(win)?;
    if status != RadioAccessStatus::Allowed {
        return Err(t!("radio.switch_denied", device = name).into());
    }

    let now = radio.State().map_err(win)?;
    if now == target {
        Ok(t!("radio.switched", device = name, state = describe(now)).into())
    } else {
        Err(t!("radio.switch_failed", device = name, state = describe(now)).into())
    }
}

fn find(device: Device) -> windows::core::Result<Option<Radio>> {
    let radios = Radio::GetRadiosAsync()?.get()?;
    for index in 0..radios.Size()? {
        let radio = radios.GetAt(index)?;
        if radio.Kind()? == device.kind() {
            return Ok(Some(radio));
        }
    }
    Ok(None)
}

fn describe(state: RadioState) -> String {
    match state {
        RadioState::On => t!("radio.state_on").into(),
        RadioState::Off => t!("radio.state_off").into(),
        RadioState::Disabled => t!("radio.state_disabled").into(),
        _ => t!("radio.state_unknown").into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_aliases() {
        assert_eq!(Device::parse("Wi-Fi").unwrap(), Device::Wifi);
        assert_eq!(Device::parse("WLAN").unwrap(), Device::Wifi);
        assert_eq!(Device::parse("bluetooth").unwrap(), Device::Bluetooth);
        assert!(Device::parse("light").is_err());
        assert_eq!(Command::parse("ein").unwrap(), Command::On);
        assert_eq!(Command::parse("off").unwrap(), Command::Off);
        assert!(Command::parse("maybe").is_err());
    }

    /// Read-only check against the real hardware.
    #[test]
    fn reads_bluetooth_status_without_changing_it() {
        match run(Device::Bluetooth, Command::Status) {
            Ok(message) => assert!(message.contains("Bluetooth"), "{message}"),
            // No adapter or no permission on this machine: still a readable message.
            Err(message) => assert!(!message.is_empty()),
        }
    }
}
