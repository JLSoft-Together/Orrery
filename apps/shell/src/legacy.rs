use std::path::{Path, PathBuf};

use crate::vault::{LegacyKey, LegacySource};

pub const ELECTRON_USER_DATA: &str = "Agent Monitor";

fn var(k: &str) -> Option<String> {
    std::env::var(k).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

pub fn default_user_data(appdata: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    if cfg!(windows) {
        appdata.map(|a| Path::new(a).join(ELECTRON_USER_DATA))
    } else {
        home.map(|h| Path::new(h).join("Library").join("Application Support").join(ELECTRON_USER_DATA))
    }
}

pub fn electron_user_data() -> Option<PathBuf> {
    if let Some(p) = var("CCM_ELECTRON_USER_DATA") {
        return Some(PathBuf::from(p));
    }
    if cfg!(debug_assertions) {
        return None;
    }
    default_user_data(var("APPDATA").as_deref(), var("HOME").as_deref())
}

pub fn keyring_service() -> String {
    var("CCM_KEYRING_SERVICE").unwrap_or_else(|| if cfg!(debug_assertions) { "Orrery Dev".into() } else { "Orrery".into() })
}

#[cfg(any(windows, test))]
pub fn local_state_key(text: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let b64 = v.get("os_crypt")?.get("encrypted_key")?.as_str()?;
    let raw = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    raw.strip_prefix(b"DPAPI").map(<[u8]>::to_vec)
}

#[cfg(windows)]
fn dpapi(data: &[u8]) -> Option<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB { cbData: u32::try_from(data.len()).ok()?, pbData: data.as_ptr().cast_mut() };
    let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    let ok = unsafe { CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), 0, &mut out) };
    if ok == 0 || out.pbData.is_null() {
        return None;
    }
    let plain = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe { LocalFree(out.pbData.cast()) };
    Some(plain)
}

#[cfg(windows)]
fn load(user_data: &Path) -> Option<LegacyKey> {
    let text = std::fs::read_to_string(user_data.join("Local State")).ok()?;
    let key = dpapi(&local_state_key(&text)?)?;
    Some(LegacyKey::Gcm(key.try_into().ok()?))
}

#[cfg(target_os = "macos")]
fn load(user_data: &Path) -> Option<LegacyKey> {
    if !user_data.join("Local State").is_file() {
        return None;
    }
    let service = var("CCM_ELECTRON_KEYCHAIN_SERVICE").unwrap_or_else(|| "Orrery Safe Storage".into());
    let password = keyring::Entry::new(&service, "Orrery Key").ok()?.get_password().ok()?;
    Some(LegacyKey::Cbc(crate::vault::mac_key(&password)))
}

pub fn source() -> LegacySource {
    Box::new(|| electron_user_data().and_then(|dir| load(&dir)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn the_electron_profile_sits_under_agent_monitor() {
        let p = default_user_data(Some("C:\\Users\\u\\AppData\\Roaming"), Some("/Users/u")).unwrap();
        assert!(p.ends_with(ELECTRON_USER_DATA));
        assert!(default_user_data(None, None).is_none());
    }

    #[test]
    fn debug_builds_never_read_the_real_profile_or_keyring() {
        if std::env::var("CCM_ELECTRON_USER_DATA").is_err() {
            assert!(electron_user_data().is_none());
        }
        if std::env::var("CCM_KEYRING_SERVICE").is_err() {
            assert_eq!(keyring_service(), "Orrery Dev");
        }
    }

    #[test]
    fn local_state_yields_the_dpapi_blob() {
        let blob = base64::engine::general_purpose::STANDARD.encode(b"DPAPIsecret");
        let text = format!(r#"{{"os_crypt":{{"encrypted_key":"{blob}"}}}}"#);
        assert_eq!(local_state_key(&text), Some(b"secret".to_vec()));
        assert!(local_state_key("{}").is_none());
        assert!(local_state_key(r#"{"os_crypt":{"encrypted_key":"bm9wZQ=="}}"#).is_none());
        assert!(local_state_key("not json").is_none());
    }
}
