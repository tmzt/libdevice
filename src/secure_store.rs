//! **Secure store** (feature `secure-store`): small named secrets on the
//! desktop, with the same API as libmobile's on Android, so libmorph only
//! wraps them.
//!
//! * [`SecureStore::keychain`] (macOS): one generic-password item per blob
//!   in the login Keychain, service `<service>`, account the blob's name.
//! * [`SecureStore::memory`]: a map per store, empty when made, for emulated
//!   and scripted runs.
//!
//! A cargo-built binary is ad-hoc signed with a new hash on each build, so
//! after a rebuild macOS asks once to allow access. From a session that
//! cannot show that prompt (ssh, an agent's shell) writes fail with
//! `errSecInteractionNotAllowed` (-25308), reported as
//! [`SecureStoreError::Platform`].

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

/// Longest blob name, in bytes.
pub const MAX_NAME_LEN: usize = 64;

/// Largest blob, in bytes.
pub const MAX_VALUE_LEN: usize = 64 * 1024;

/// Named byte blobs, kept by the platform. Cheap to clone, `Send + Sync`.
#[derive(Clone)]
pub struct SecureStore(Backend);

#[derive(Clone)]
enum Backend {
    #[cfg(target_os = "macos")]
    Keychain(Arc<str>),
    Memory(Arc<Mutex<HashMap<String, Vec<u8>>>>),
}

impl fmt::Debug for SecureStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SecureStore")
            .field(&self.security_level())
            .finish()
    }
}

/// How a blob is stored.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct PutOptions {
    /// Keep the blob unreadable while the device is locked. Accepted but
    /// not enforced on macOS: per-item accessibility needs the
    /// data-protection Keychain, which an unsigned binary cannot use, so a
    /// login Keychain item is readable whenever the user is logged in.
    pub unlocked_only: bool,
}

impl PutOptions {
    #[must_use]
    pub fn unlocked_only(mut self, unlocked_only: bool) -> Self {
        self.unlocked_only = unlocked_only;
        self
    }
}

/// Why a secure store call failed. The same variants as libmobile's.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecureStoreError {
    /// Not 1 to 64 of `[A-Za-z0-9._-]`, or starts with `.`.
    InvalidName,
    /// Over [`MAX_VALUE_LEN`].
    TooLarge,
    /// The platform's key storage refused (Android Keystore; unused here).
    Keystore(String),
    /// The device is locked and the blob needs it unlocked.
    DeviceLocked,
    /// Reading or writing the blob failed.
    Io(String),
    /// The blob is malformed or fails to decrypt.
    Corrupt(String),
    /// The platform failed (the Keychain).
    Platform(String),
}

impl fmt::Display for SecureStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => write!(
                f,
                "secure store: names are 1 to {MAX_NAME_LEN} of [A-Za-z0-9._-], not starting with '.'"
            ),
            Self::TooLarge => write!(f, "secure store: blobs are at most {MAX_VALUE_LEN} bytes"),
            Self::Keystore(detail) => write!(f, "secure store: keystore: {detail}"),
            Self::DeviceLocked => write!(f, "secure store: the device is locked"),
            Self::Io(detail) => write!(f, "secure store: io: {detail}"),
            Self::Corrupt(detail) => write!(f, "secure store: corrupt: {detail}"),
            Self::Platform(detail) => write!(f, "secure store: platform: {detail}"),
        }
    }
}

impl std::error::Error for SecureStoreError {}

/// Where the secrets are kept. The variants are per platform; libmobile's
/// are `Software`, `TrustedEnvironment` and `StrongBox`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SecurityLevel {
    Keychain,
    Memory,
}

impl SecureStore {
    /// The login Keychain, items under `service`.
    #[cfg(target_os = "macos")]
    pub fn keychain(service: &str) -> Self {
        Self(Backend::Keychain(service.into()))
    }

    /// A map of its own, empty, gone when the last clone drops.
    pub fn memory() -> Self {
        Self(Backend::Memory(Arc::default()))
    }

    /// The blob called `name`, or `None` if there is none.
    pub fn get(&self, name: &str) -> Result<Option<Vec<u8>>, SecureStoreError> {
        check_name(name)?;
        match &self.0 {
            #[cfg(target_os = "macos")]
            Backend::Keychain(service) => keychain::get(service, name).map_err(platform),
            Backend::Memory(map) => Ok(lock(map).get(name).cloned()),
        }
    }

    /// Store `value` as the blob called `name`, replacing any earlier one.
    pub fn put(&self, name: &str, value: &[u8]) -> Result<(), SecureStoreError> {
        self.put_with(name, value, PutOptions::default())
    }

    /// [`put`](Self::put) with options; see [`PutOptions`].
    pub fn put_with(
        &self,
        name: &str,
        value: &[u8],
        options: PutOptions,
    ) -> Result<(), SecureStoreError> {
        check_name(name)?;
        if value.len() > MAX_VALUE_LEN {
            return Err(SecureStoreError::TooLarge);
        }
        let _ = options;
        match &self.0 {
            #[cfg(target_os = "macos")]
            Backend::Keychain(service) => keychain::put(service, name, value).map_err(platform),
            Backend::Memory(map) => {
                lock(map).insert(name.to_owned(), value.to_vec());
                Ok(())
            }
        }
    }

    /// Remove the blob called `name`. `true` if there was one.
    pub fn delete(&self, name: &str) -> Result<bool, SecureStoreError> {
        check_name(name)?;
        match &self.0 {
            #[cfg(target_os = "macos")]
            Backend::Keychain(service) => keychain::delete(service, name).map_err(platform),
            Backend::Memory(map) => Ok(lock(map).remove(name).is_some()),
        }
    }

    /// Where the secrets are kept.
    pub fn security_level(&self) -> SecurityLevel {
        match &self.0 {
            #[cfg(target_os = "macos")]
            Backend::Keychain(_) => SecurityLevel::Keychain,
            Backend::Memory(_) => SecurityLevel::Memory,
        }
    }
}

fn lock(
    map: &Mutex<HashMap<String, Vec<u8>>>,
) -> std::sync::MutexGuard<'_, HashMap<String, Vec<u8>>> {
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(target_os = "macos")]
fn platform(detail: String) -> SecureStoreError {
    SecureStoreError::Platform(detail)
}

/// 1 to [`MAX_NAME_LEN`] of `[A-Za-z0-9._-]`, not starting with `.`.
fn check_name(name: &str) -> Result<(), SecureStoreError> {
    let valid = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if valid {
        Ok(())
    } else {
        Err(SecureStoreError::InvalidName)
    }
}

/// The login Keychain through `security-framework`: generic passwords,
/// `service` and `account`. Shared with the key store's vault.
#[cfg(target_os = "macos")]
pub(crate) mod keychain {
    use security_framework::passwords;

    /// `errSecItemNotFound`.
    const NOT_FOUND: i32 = -25300;

    fn describe(error: security_framework::base::Error) -> String {
        format!("keychain: {error} ({})", error.code())
    }

    pub(crate) fn get(service: &str, account: &str) -> Result<Option<Vec<u8>>, String> {
        match passwords::get_generic_password(service, account) {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.code() == NOT_FOUND => Ok(None),
            Err(error) => Err(describe(error)),
        }
    }

    pub(crate) fn put(service: &str, account: &str, value: &[u8]) -> Result<(), String> {
        passwords::set_generic_password(service, account, value).map_err(describe)
    }

    pub(crate) fn delete(service: &str, account: &str) -> Result<bool, String> {
        match passwords::delete_generic_password(service, account) {
            Ok(()) => Ok(true),
            Err(error) if error.code() == NOT_FOUND => Ok(false),
            Err(error) => Err(describe(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_sizes() {
        let store = SecureStore::memory();
        for ok in ["a", "noise.static", "server", "A-b_c.9", &"x".repeat(64)] {
            assert_eq!(store.get(ok), Ok(None), "{ok}");
        }
        for bad in ["", ".hidden", "a/b", "a b", "ü", &"x".repeat(65)] {
            assert_eq!(
                store.get(bad),
                Err(SecureStoreError::InvalidName),
                "{bad:?}"
            );
        }
        assert_eq!(store.put("k", &vec![0; MAX_VALUE_LEN]), Ok(()));
        assert_eq!(
            store.put("k", &vec![0; MAX_VALUE_LEN + 1]),
            Err(SecureStoreError::TooLarge)
        );
    }

    #[test]
    fn memory_round_trip_and_separation() {
        let store = SecureStore::memory();
        let other = store.clone();
        store.put("k", b"v").unwrap();
        assert_eq!(other.get("k"), Ok(Some(b"v".to_vec())));
        assert_eq!(SecureStore::memory().get("k"), Ok(None));
        assert_eq!(store.delete("k"), Ok(true));
        assert_eq!(store.delete("k"), Ok(false));
        assert_eq!(store.security_level(), SecurityLevel::Memory);
    }

    /// Writes to the login Keychain (service `libdevice.test`), so it runs
    /// only when asked, from a Terminal: `cargo test --features secure-store
    /// -- --ignored keychain`.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "uses the login Keychain"]
    fn keychain_round_trip() {
        let store = SecureStore::keychain("libdevice.test");
        let _ = store.delete("k");
        assert_eq!(store.get("k"), Ok(None));
        store.put("k", &[1, 2, 3]).unwrap();
        assert_eq!(store.get("k"), Ok(Some(vec![1, 2, 3])));
        store.put("k", &[4]).unwrap();
        assert_eq!(store.get("k"), Ok(Some(vec![4])));
        assert_eq!(store.delete("k"), Ok(true));
        assert_eq!(store.delete("k"), Ok(false));
    }
}
