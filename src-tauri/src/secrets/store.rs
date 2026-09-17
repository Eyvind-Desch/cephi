//! Storage backend: Windows Credential Manager.
//! Entries appear in *Anmeldeinformationsverwaltung → Windows-Anmeldeinformationen* as `Orbit/<name>`.

use std::ptr;

use rust_i18n::t;
use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FILETIME};
use windows_sys::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

const TARGET_PREFIX: &str = "Orbit/";
/// Windows limit for generic credential blobs.
const MAX_SECRET_BYTES: usize = 2560;

pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    (2..=64).contains(&name.len())
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn is_set(name: &str) -> bool {
    matches!(get(name), Ok(Some(_)))
}

pub fn get(name: &str) -> Result<Option<String>, String> {
    let target = wide(&target_name(name)?);
    let mut credential: *mut CREDENTIALW = ptr::null_mut();

    // SAFETY: `target` is a valid NUL-terminated UTF-16 string; on success `credential` points to
    // memory owned by Windows that is released with `CredFree` below.
    unsafe {
        if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0 {
            return match GetLastError() {
                ERROR_NOT_FOUND => Ok(None),
                code => Err(t!("secrets.read_failed", code = code).into()),
            };
        }
        let blob = std::slice::from_raw_parts((*credential).CredentialBlob, (*credential).CredentialBlobSize as usize);
        let value = String::from_utf8(blob.to_vec());
        CredFree(credential.cast());
        value.map(Some).map_err(|_| t!("secrets.not_text").into())
    }
}

pub fn set(name: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(t!("secrets.empty").into());
    }
    if value.len() > MAX_SECRET_BYTES {
        return Err(t!("secrets.too_long").into());
    }

    let mut target = wide(&target_name(name)?);
    let mut user = wide("Orbit");
    let mut blob = value.as_bytes().to_vec();

    let credential = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target.as_mut_ptr(),
        Comment: ptr::null_mut(),
        LastWritten: FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        },
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: ptr::null_mut(),
        TargetAlias: ptr::null_mut(),
        UserName: user.as_mut_ptr(),
    };

    // SAFETY: all pointers in `credential` reference live buffers for the duration of the call.
    if unsafe { CredWriteW(&credential, 0) } == 0 {
        let code = unsafe { GetLastError() };
        return Err(t!("secrets.save_failed", code = code).into());
    }
    Ok(())
}

pub fn delete(name: &str) -> Result<(), String> {
    let target = wide(&target_name(name)?);
    // SAFETY: `target` is a valid NUL-terminated UTF-16 string.
    unsafe {
        if CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) == 0 {
            let code = GetLastError();
            if code != ERROR_NOT_FOUND {
                return Err(t!("secrets.delete_failed", code = code).into());
            }
        }
    }
    Ok(())
}

fn target_name(name: &str) -> Result<String, String> {
    if is_valid_name(name) {
        Ok(format!("{TARGET_PREFIX}{name}"))
    } else {
        Err(t!("secrets.invalid_name", name = name).into())
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Tests that write to the real Credential Manager run one at a time: concurrent writes from
    /// parallel test threads fail intermittently.
    pub(crate) static CREDENTIAL_MANAGER: Mutex<()> = Mutex::new(());

    #[test]
    fn roundtrip_in_credential_manager() {
        let _serial = CREDENTIAL_MANAGER.lock().unwrap_or_else(|e| e.into_inner());
        // Unique name: the test touches the real Credential Manager, runs must not interfere.
        let name = &format!("orbit_test_{}", uuid::Uuid::new_v4().simple());
        // Non-ASCII on purpose: the blob must round-trip as UTF-8.
        set(name, "  value-äöü  ").unwrap();
        assert_eq!(get(name).unwrap().as_deref(), Some("value-äöü"));
        delete(name).unwrap();
        assert_eq!(get(name).unwrap(), None);
        delete(name).unwrap(); // deleting twice is fine
    }

    #[test]
    fn rejects_invalid_names() {
        assert!(get("Bad-Name").is_err());
        assert!(set("x", "v").is_err());
    }
}
