//! Start with Windows: a value in `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! Cephi started this way gets [`AUTOSTART_ARG`] and stays in the tray until the hotkey is pressed.

use std::os::windows::ffi::OsStrExt;

use rust_i18n::t;
use serde::Serialize;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

/// Passed by Windows on login (part of the registered command line).
pub const AUTOSTART_ARG: &str = "--autostart";

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Cephi";

/// Mirrored in `src/lib/ipc.ts` (`AutostartState`).
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartState {
    pub enabled: bool,
    /// Development builds need the dev server, so they can't start on their own.
    pub available: bool,
}

pub fn state() -> AutostartState {
    AutostartState {
        enabled: registered_command().is_some_and(|command| command == own_command()),
        available: !cfg!(debug_assertions),
    }
}

pub fn set(enabled: bool) -> Result<AutostartState, String> {
    if enabled && cfg!(debug_assertions) {
        return Err(t!("autostart.dev_build").into());
    }
    let key = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    let status = if enabled {
        let command = wide(&own_command());
        // SAFETY: all strings are NUL-terminated UTF-16 buffers that outlive the call.
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        // SAFETY: see above.
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
        // Not registered is fine when disabling.
        if status == windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND {
            ERROR_SUCCESS
        } else {
            status
        }
    };
    if status != ERROR_SUCCESS {
        return Err(t!("autostart.failed", code = status).into());
    }
    log::info!("autostart {}", if enabled { "enabled" } else { "disabled" });
    Ok(state())
}

/// The command line Windows should run: this executable, quoted, plus the autostart flag.
fn own_command() -> String {
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    format!("\"{exe}\" {AUTOSTART_ARG}")
}

fn registered_command() -> Option<String> {
    let key = wide(RUN_KEY);
    let name = wide(VALUE_NAME);
    let mut buffer = vec![0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: the buffer and its size in bytes are passed together.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let chars = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..chars]))
}

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_quotes_the_path_and_adds_the_flag() {
        let command = own_command();
        assert!(command.starts_with('"'));
        assert!(command.ends_with(AUTOSTART_ARG));
    }

    #[test]
    fn development_builds_refuse_to_register() {
        if cfg!(debug_assertions) {
            assert!(set(true).is_err());
            assert!(!state().available);
        }
    }
}
