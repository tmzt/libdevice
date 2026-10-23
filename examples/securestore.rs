//! Counts its runs in the secure store and keeps a test key in the key
//! store, in the macOS login Keychain. A command-line program with no
//! window, so it runs from a Terminal:
//!
//! ```text
//! cargo run --example securestore --features secure-store
//! cargo run --example securestore --features secure-store -- --reset
//! security find-generic-password -s libdevice.example -a counter
//! security find-generic-password -s libdevice.example.keys -a device
//! ```
//!
//! Each run reads blob `counter` (a little-endian `u64`), prints it plus one
//! and stores that, then loads key `device`, generating it on the first run,
//! and prints its fingerprint. It also tries a key with
//! `FLAG_DEVICE_UNLOCK_REQUIRED`, which the login Keychain cannot enforce,
//! so it prints `UnsupportedFlags(1)`. `--reset` deletes both.
//!
//! A cargo build is signed with a new hash each time, so after a rebuild
//! macOS asks once to allow access to the items. To stop that during
//! development, sign the binary with a stable local identity after each
//! build: create a self-signed code-signing certificate in Keychain Access
//! (Certificate Assistant, "Create a Certificate", type "Code Signing"), then
//! `codesign -f -s "<its name>" target/debug/examples/securestore`.

#[cfg(target_os = "macos")]
fn main() {
    use libdevice::key_store::{self, FLAG_DEVICE_UNLOCK_REQUIRED, KeyStoreError, Vault};
    use libdevice::secure_store::SecureStore;

    const NAMESPACE: &str = "libdevice.example";
    let store = SecureStore::keychain(NAMESPACE);
    let vault = Vault::named(NAMESPACE).expect("a valid namespace");

    if std::env::args().any(|arg| arg == "--reset") {
        println!("counter deleted: {:?}", store.delete("counter"));
        println!("key deleted: {:?}", key_store::delete(&vault, "device"));
        return;
    }

    let count = match store.get("counter") {
        Ok(blob) => decode(blob.as_deref()) + 1,
        Err(error) => return eprintln!("counter: {error}"),
    };
    match store.put("counter", &count.to_le_bytes()) {
        Ok(()) => println!("run {count}"),
        Err(error) => return eprintln!("counter: {error}"),
    }

    match key_store::public::<example_key::ExampleKey>(&vault, "device") {
        Ok(public) => println!("key device: {public:016x} (loaded)"),
        Err(KeyStoreError::NotFound) => {
            match key_store::generate::<example_key::ExampleKey>(&vault, "device", 0) {
                Ok(public) => println!("key device: {public:016x} (generated)"),
                Err(error) => eprintln!("key device: {error}"),
            }
        }
        Err(error) => eprintln!("key device: {error:?}"),
    }
    println!(
        "key with FLAG_DEVICE_UNLOCK_REQUIRED: {:?}",
        key_store::generate::<example_key::ExampleKey>(
            &vault,
            "device-unlocked",
            FLAG_DEVICE_UNLOCK_REQUIRED
        )
    );
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("securestore: the Keychain is macOS only");
}

/// The stored count, or 0 for none. A blob of the wrong length is 0 too.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn decode(blob: Option<&[u8]>) -> u64 {
    blob.and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .unwrap_or(0)
}

/// A test adapter: 32 random bytes behind a pointer, with a `u64`
/// fingerprint as its public value (enough for a test).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod example_key {
    use libdevice::key_store::{SealToken, SecretKey};
    use std::hash::{DefaultHasher, Hash, Hasher};
    use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

    #[derive(Zeroize, ZeroizeOnDrop)]
    pub struct ExampleKey(Box<[u8; 32]>);

    impl SecretKey for ExampleKey {
        type Public = u64;
        const KIND: &'static str = "libdevice.example.test.v1";

        fn generate(_: &SealToken) -> Self {
            let mut key = Self(Box::new([0; 32]));
            if let Err(error) = std::fs::File::open("/dev/urandom")
                .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut key.0[..]))
            {
                panic!("example key: no randomness: {error}");
            }
            key
        }

        fn seal(&self, _: &SealToken) -> Zeroizing<Vec<u8>> {
            Zeroizing::new(self.0.to_vec())
        }

        fn unseal(_: &str, bytes: &[u8], _: &SealToken) -> Result<Self, String> {
            if bytes.len() != 32 {
                return Err(format!("{} bytes, not 32", bytes.len()));
            }
            let mut key = Self(Box::new([0; 32]));
            key.0.copy_from_slice(bytes);
            Ok(key)
        }

        fn public(&self) -> u64 {
            let mut hasher = DefaultHasher::new();
            self.0.hash(&mut hasher);
            hasher.finish()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn counter_decodes() {
        assert_eq!(super::decode(None), 0);
        assert_eq!(super::decode(Some(&[1, 2])), 0);
        assert_eq!(super::decode(Some(&41u64.to_le_bytes())), 41);
    }
}
