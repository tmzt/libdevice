//! libdevice's half of the key store: where sealed envelopes live.
//!
//! * `Vault::named(namespace)` (macOS): the login Keychain, service
//!   `<namespace>.keys`, account the key's name, data `[flags][envelope]`.
//!   It enforces no flags: per-item lock state needs the data-protection
//!   Keychain, which an unsigned binary cannot use, so
//!   `FLAG_DEVICE_UNLOCK_REQUIRED` is refused rather than accepted without
//!   being enforced.
//! * `Vault::memory()` (feature `test-vault`, for tests only): a map per
//!   instance, with a simulated device lock.
//!
//! The engine (`key_store.rs`) is the same file as libmobile's. The checks
//! below name this crate, so they live here.
//!
//! A `SealToken` cannot be made outside the crate:
//!
//! ```compile_fail,E0423
//! let token = libdevice::key_store::SealToken(());
//! ```
//!
//! nor kept past the call that lends it:
//!
//! ```compile_fail,E0521
//! use libdevice::key_store::SealToken;
//! static mut KEPT: Option<&'static SealToken> = None;
//! fn keep(token: &SealToken) {
//!     unsafe { KEPT = Some(token) };
//! }
//! ```
//!
//! and `with_key` cannot hand the key out of its closure:
//!
//! ```compile_fail
//! use libdevice::key_store::{SecretKey, Vault, with_key};
//! fn leak<K: SecretKey>(vault: &Vault) -> &K {
//!     with_key::<K, _>(vault, "k", |key| key).unwrap()
//! }
//! ```

#![cfg_attr(
    not(feature = "test-vault"),
    doc = "Without `test-vault` there is no memory vault:\n\n```compile_fail,E0599\nlet vault = libdevice::key_store::Vault::memory();\n```"
)]
// A host build without `test-vault` has no vault at all.
#![cfg_attr(not(any(target_os = "macos", feature = "test-vault")), allow(unused))]

#[cfg(feature = "test-vault")]
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(feature = "test-vault")]
use std::sync::Mutex;
#[cfg(feature = "test-vault")]
use std::sync::atomic::{AtomicBool, Ordering};

use zeroize::Zeroizing;

#[cfg(feature = "test-vault")]
use super::FLAG_DEVICE_UNLOCK_REQUIRED;
use super::KeyStoreError;

/// A stored item: its flags and its sealed envelope.
pub(super) type Stored = (u8, Zeroizing<Vec<u8>>);

/// A thin handle over the platform's storage; not a security boundary.
/// Cheap to clone, `'static`, usable from any thread.
#[derive(Clone)]
pub struct Vault(Inner);

#[derive(Clone)]
enum Inner {
    #[cfg(target_os = "macos")]
    Named(Arc<str>),
    #[cfg(feature = "test-vault")]
    Memory(Arc<Memory>),
}

#[cfg(feature = "test-vault")]
#[derive(Default)]
struct Memory {
    items: Mutex<HashMap<String, Stored>>,
    locked: AtomicBool,
}

impl Vault {
    /// The namespace's vault: Keychain service `<namespace>.keys`.
    /// Namespaces follow the key name rule.
    #[cfg(target_os = "macos")]
    pub fn named(namespace: &str) -> Result<Vault, KeyStoreError> {
        super::check_name(namespace)?;
        Ok(Vault(Inner::Named(namespace.into())))
    }

    /// For tests: a map of its own, empty, gone when the last clone drops.
    #[cfg(feature = "test-vault")]
    pub fn memory() -> Vault {
        Vault(Inner::Memory(Arc::default()))
    }

    /// For tests: while set, keys with `FLAG_DEVICE_UNLOCK_REQUIRED` give
    /// `DeviceLocked`. No effect on a named vault.
    #[cfg(feature = "test-vault")]
    pub fn set_device_locked(&self, locked: bool) {
        #[allow(irrefutable_let_patterns)]
        if let Inner::Memory(memory) = &self.0 {
            memory.locked.store(locked, Ordering::Relaxed);
        }
    }

    /// Flags this vault enforces.
    pub(super) fn supported_flags(&self) -> u8 {
        match self.0 {
            #[cfg(target_os = "macos")]
            Inner::Named(_) => 0,
            #[cfg(feature = "test-vault")]
            Inner::Memory(_) => FLAG_DEVICE_UNLOCK_REQUIRED,
        }
    }

    /// The stored flags and envelope, or `None`.
    pub(super) fn get(&self, name: &str) -> Result<Option<Stored>, KeyStoreError> {
        match self.0 {
            #[cfg(target_os = "macos")]
            Inner::Named(ref namespace) => keychain_vault::get(namespace, name),
            #[cfg(feature = "test-vault")]
            Inner::Memory(ref memory) => {
                let items = memory.items.lock().unwrap_or_else(|e| e.into_inner());
                match items.get(name) {
                    Some((flags, _))
                        if flags & FLAG_DEVICE_UNLOCK_REQUIRED != 0
                            && memory.locked.load(Ordering::Relaxed) =>
                    {
                        Err(KeyStoreError::DeviceLocked)
                    }
                    Some((flags, envelope)) => Ok(Some((*flags, envelope.clone()))),
                    None => Ok(None),
                }
            }
        }
    }

    pub(super) fn put(&self, name: &str, flags: u8, envelope: &[u8]) -> Result<(), KeyStoreError> {
        match self.0 {
            #[cfg(target_os = "macos")]
            Inner::Named(ref namespace) => keychain_vault::put(namespace, name, flags, envelope),
            #[cfg(feature = "test-vault")]
            Inner::Memory(ref memory) => {
                let mut items = memory.items.lock().unwrap_or_else(|e| e.into_inner());
                items.insert(name.to_owned(), (flags, Zeroizing::new(envelope.to_vec())));
                Ok(())
            }
        }
    }

    pub(super) fn delete(&self, name: &str) -> Result<bool, KeyStoreError> {
        match self.0 {
            #[cfg(target_os = "macos")]
            Inner::Named(ref namespace) => keychain_vault::delete(namespace, name),
            #[cfg(feature = "test-vault")]
            Inner::Memory(ref memory) => {
                let mut items = memory.items.lock().unwrap_or_else(|e| e.into_inner());
                Ok(items.remove(name).is_some())
            }
        }
    }
}

/// The login Keychain, one generic password per key.
#[cfg(target_os = "macos")]
mod keychain_vault {
    use super::super::KeyStoreError;
    use super::Stored;
    use crate::secure_store::keychain;
    use zeroize::Zeroizing;

    fn service(namespace: &str) -> String {
        format!("{namespace}.keys")
    }

    pub(super) fn get(namespace: &str, name: &str) -> Result<Option<Stored>, KeyStoreError> {
        let Some(data) =
            keychain::get(&service(namespace), name).map_err(KeyStoreError::Platform)?
        else {
            return Ok(None);
        };
        let data = Zeroizing::new(data);
        let (&flags, envelope) = data
            .split_first()
            .ok_or_else(|| KeyStoreError::Corrupt("empty item".into()))?;
        Ok(Some((flags, Zeroizing::new(envelope.to_vec()))))
    }

    pub(super) fn put(
        namespace: &str,
        name: &str,
        flags: u8,
        envelope: &[u8],
    ) -> Result<(), KeyStoreError> {
        let mut data = Zeroizing::new(Vec::with_capacity(1 + envelope.len()));
        data.push(flags);
        data.extend_from_slice(envelope);
        keychain::put(&service(namespace), name, &data).map_err(KeyStoreError::Platform)
    }

    pub(super) fn delete(namespace: &str, name: &str) -> Result<bool, KeyStoreError> {
        keychain::delete(&service(namespace), name).map_err(KeyStoreError::Platform)
    }
}

/// Writes to the login Keychain (service `libdevice.test.keys`), so it runs
/// only when asked, from a Terminal: `cargo test --features test-vault --
/// --ignored keychain`.
#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::super::{FLAG_DEVICE_UNLOCK_REQUIRED, KeyStoreError};
    use super::Vault;

    #[test]
    #[ignore = "uses the login Keychain"]
    fn keychain_vault_round_trip() {
        let vault = Vault::named("libdevice.test").unwrap();
        let _ = vault.delete("k");
        assert_eq!(vault.get("k"), Ok(None));
        vault.put("k", 0, &[3, b'a', b'b', b'c', 9]).unwrap();
        let (flags, envelope) = vault.get("k").unwrap().unwrap();
        assert_eq!(
            (flags, envelope.as_slice()),
            (0, &[3, b'a', b'b', b'c', 9][..])
        );
        assert_eq!(vault.delete("k"), Ok(true));
        assert_eq!(vault.supported_flags() & FLAG_DEVICE_UNLOCK_REQUIRED, 0);
        let _ = KeyStoreError::NotFound;
    }
}
