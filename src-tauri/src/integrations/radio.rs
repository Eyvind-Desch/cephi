//! Bluetooth and Wi-Fi radios via `Windows.Devices.Radios`.
//!
//! Every answer reflects the *actual* radio state read from Windows, so the model never has to
//! guess whether something is on.

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
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().replace(['-', ' '], "").as_str() {
            "bluetooth" | "bt" => Ok(Self::Bluetooth),
            "wlan" | "wifi" | "wireless" => Ok(Self::Wifi),
            other => Err(format!("Unbekanntes Funkgerät \"{other}\" (möglich: bluetooth, wlan).")),
        }
    }

    fn kind(self) -> RadioKind {
        match self {
            Self::Bluetooth => RadioKind::Bluetooth,
            Self::Wifi => RadioKind::WiFi,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Bluetooth => "Bluetooth",
            Self::Wifi => "WLAN",
        }
    }
}

impl Command {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "on" | "an" | "ein" => Ok(Self::On),
            "off" | "aus" => Ok(Self::Off),
            "toggle" | "umschalten" => Ok(Self::Toggle),
            "status" => Ok(Self::Status),
            other => Err(format!(
                "Unbekannter Befehl \"{other}\" (möglich: on, off, toggle, status)."
            )),
        }
    }
}

/// Blocking: call from a blocking thread.
pub fn run(device: Device, command: Command) -> Result<String, String> {
    let win = |e: windows::core::Error| format!("Windows-Fehler: {}", e.message());

    // SAFETY: initializing COM for this thread; "already initialized" results are fine.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    if Radio::RequestAccessAsync().and_then(|op| op.get()).map_err(win)? != RadioAccessStatus::Allowed {
        return Err("Windows erlaubt keinen Zugriff auf Funkgeräte. \
            Einstellungen → Datenschutz → Funkgeräte prüfen."
            .into());
    }

    let radio = find(device)
        .map_err(win)?
        .ok_or_else(|| format!("Kein {}-Adapter gefunden.", device.label()))?;
    let name = device.label();
    let current = radio.State().map_err(win)?;

    if current == RadioState::Disabled {
        return Err(format!(
            "{name} ist per Hardware oder Flugmodus deaktiviert und kann nicht geschaltet werden."
        ));
    }

    let target = match command {
        Command::Status => return Ok(format!("{name} ist {}.", describe(current))),
        Command::On => RadioState::On,
        Command::Off => RadioState::Off,
        Command::Toggle if current == RadioState::On => RadioState::Off,
        Command::Toggle => RadioState::On,
    };

    if current == target {
        return Ok(format!("{name} war bereits {} (geprüft).", describe(current)));
    }

    let status = radio.SetStateAsync(target).and_then(|op| op.get()).map_err(win)?;
    if status != RadioAccessStatus::Allowed {
        return Err(format!("Windows hat das Umschalten von {name} verweigert."));
    }

    let now = radio.State().map_err(win)?;
    if now == target {
        Ok(format!("{name} ist jetzt {}.", describe(now)))
    } else {
        Err(format!(
            "{name} konnte nicht umgeschaltet werden, ist weiterhin {}.",
            describe(now)
        ))
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

fn describe(state: RadioState) -> &'static str {
    match state {
        RadioState::On => "an",
        RadioState::Off => "aus",
        RadioState::Disabled => "deaktiviert",
        _ => "in unbekanntem Zustand",
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
        assert!(Device::parse("licht").is_err());
        assert_eq!(Command::parse("ein").unwrap(), Command::On);
        assert!(Command::parse("vielleicht").is_err());
    }

    /// Read-only check against the real hardware.
    #[test]
    fn reads_bluetooth_status_without_changing_it() {
        match run(Device::Bluetooth, Command::Status) {
            Ok(message) => assert!(message.starts_with("Bluetooth ist"), "{message}"),
            Err(message) => assert!(message.contains("Adapter") || message.contains("Zugriff"), "{message}"),
        }
    }
}
