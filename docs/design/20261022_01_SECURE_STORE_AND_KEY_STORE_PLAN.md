---
title: Secure store and key store on the desktop
date: 2026-10-22
status: Draft
covers:
  - A `secure-store` feature: libdevice's first platform code, kept out of
    the default build
  - "`SecureStore`: named byte blobs in the macOS login Keychain, or in
    memory"
  - "`KeyStore`: adapter-defined keys (`SecretKey`, `SealToken`,
    `KeyHandle`), with the same API as libmobile's key store, in a Keychain
    service of their own, or in memory"
  - A command-line example that runs from a terminal
sources:
  - Owner (2026-10-22), in the libmorph session. The desktop backends of
    libmorph's secure store belong in libdevice ("libmobile and libdevice
    will both need wrapped implementations with the same API"), and the
    layering rule now covers libdevice: "that should be extended to
    libdevice with the same rule, libmorph is a wrapper only (with a few
    exceptions)".
  - libmorph docs/design/20261022_04_OPAQUE_KEYS_PLAN.md, amendment "a key
    store of its own, built in libmobile and libdevice first". This plan is
    its step 2.
  - libmobile docs/design/20261022_03_SECURE_STORE_PLAN.md (the blob store
    on Android) and docs/design/20261022_04_KEY_STORE_PLAN.md (the key store
    API this plan repeats).
  - libmorph's unpushed `secure-store` branch, whose Keychain and memory
    backends (`src/secure_store.rs`) move here.
  - macOS Security framework through the `security-framework` crate
    (`passwords::{get,set,delete}_generic_password`, `errSecItemNotFound`
    -25300, `errSecInteractionNotAllowed` -25308).
  - The plan rules in libmorph's AGENTS.md (critical rules 0 to 4), used
    here because libdevice has no AGENTS.md of its own, as in
    `20261015_01_DEVICE_PIXEL_DENSITY_PLAN.md`.
related:
  - docs/design/20261015_01_DEVICE_PIXEL_DENSITY_PLAN.md (it said libdevice
    holds only facts about the emulated device; this plan widens that, on
    the owner's instruction, for the desktop's platform services)
  - libmorph docs/design/20261022_03_SECURE_STORE_PLAN.md and
    20261022_04_OPAQUE_KEYS_PLAN.md (libmorph wraps this plan and
    libmobile's)
---

# Secure store and key store on the desktop

## 1. Goal

libdevice gives the desktop the same two stores libmobile gives Android,
with the same API, so libmorph only wraps them:

- a **blob store** for small named secrets;
- a **key store** for keys whose type an adapter defines (`SecretKey`).
  The store generates, seals, loads and validates them, and hands out a
  handle that exposes only the adapter's public value.

Both are behind a `secure-store` feature. Without it libdevice stays what
it is today: device data with no dependencies.

## 2. Backends

| Backend | Blobs | Keys | Notes |
|---|---|---|---|
| Keychain (macOS) | generic password, service `<service>`, account = name | service `<service>.keys`, account = name | the login Keychain |
| Memory (any host) | a map per store | a map per store | empty when made; for emulated and scripted runs |

- `<service>` is the caller's, for example libmorph's
  `libmorph.<SECURE_STORE_ID>`. The blob and key services differ, so a key
  and a blob can share a name.
- **Unsigned binaries.** A cargo-built binary is ad-hoc signed with a new
  hash on each build, so after a rebuild macOS asks once to allow access.
  The data-protection Keychain would need a signed app with an entitlement,
  so it is not used. From a session that cannot show that prompt (an agent's
  shell, ssh), writes fail with -25308; that is reported as `Platform`.
- **`unlocked_only` is accepted but not enforced on macOS.** Per-item
  accessibility needs the data-protection Keychain. Login Keychain items
  are readable whenever the user is logged in. The rustdoc says so.
- Linux and Windows have no backend. `SecureStore::keychain` and
  `KeyStore::keychain` exist only on macOS.

## 3. API (module `secure_store`, re-exported at the crate root)

The types match libmobile's field for field, with two changes proposed to
libmobile so that the shapes agree (section 6).

### 3.1 Blobs

```rust
pub const MAX_NAME_LEN: usize = 64;
pub const MAX_VALUE_LEN: usize = 64 * 1024;

#[derive(Clone)] // Send + Sync
pub struct SecureStore { /* backend */ }

impl SecureStore {
    #[cfg(target_os = "macos")]
    pub fn keychain(service: &str) -> Self;
    pub fn memory() -> Self;
    pub fn get(&self, name: &str) -> Result<Option<Vec<u8>>, SecureStoreError>;
    pub fn put(&self, name: &str, value: &[u8]) -> Result<(), SecureStoreError>;
    pub fn put_with(&self, name: &str, value: &[u8], options: PutOptions)
        -> Result<(), SecureStoreError>;
    pub fn delete(&self, name: &str) -> Result<bool, SecureStoreError>;
    pub fn security_level(&self) -> SecurityLevel;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct PutOptions { pub unlocked_only: bool }

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SecureStoreError {
    InvalidName, TooLarge, Keystore(String), Locked, Io(String),
    Corrupt(String), Platform(String),
}

/// Where the secrets are kept. The variants are per platform; libmobile's
/// are Software, TrustedEnvironment and StrongBox.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SecurityLevel { Keychain, Memory }
```

Names are 1 to 64 of `[A-Za-z0-9._-]` and do not start with `.`; values
are at most 64 KiB. Both are libmobile's rules.

### 3.2 Keys

```rust
pub struct SealToken(());   // only libdevice can make one
pub trait SecretKey: Sized + Send + Sync + 'static {
    type Public: Clone + Send + Sync + 'static;
    const KIND: &'static str;
    fn generate(token: SealToken) -> Self;
    fn seal(&self, token: SealToken) -> Vec<u8>;
    fn unseal(bytes: &[u8], token: SealToken) -> Result<Self, String>;
    fn public(&self) -> Self::Public;
}
pub struct KeyHandle<K: SecretKey>;  // public(), key(), Clone
#[non_exhaustive] #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyOptions { pub unlocked_only: bool }
#[non_exhaustive] #[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyStoreError { InvalidName, WrongKind(String), Corrupt(String), Locked, Platform(String) }

#[derive(Clone)] // Send + Sync
pub struct KeyStore { /* backend */ }
impl KeyStore {
    #[cfg(target_os = "macos")]
    pub fn keychain(service: &str) -> Self;     // uses `<service>.keys`
    pub fn memory() -> Self;
    pub fn generate<K: SecretKey>(&self, name: &str) -> Result<KeyHandle<K>, KeyStoreError>;
    pub fn generate_with<K: SecretKey>(&self, name: &str, options: KeyOptions)
        -> Result<KeyHandle<K>, KeyStoreError>;
    pub fn load<K: SecretKey>(&self, name: &str) -> Result<Option<KeyHandle<K>>, KeyStoreError>;
    pub fn delete(&self, name: &str) -> Result<bool, KeyStoreError>;
}
```

The sealed form is libmobile's: `[kind length: u8][kind][sealed bytes]`,
at most 64 KiB, with the kind checked before `unseal` (`WrongKind`).
Copies the store holds are cleared after use. `generate` takes no RNG; the
adapter uses its own.

Every call is synchronous and may be made from any thread.

## 4. Steps

1. **Types and the memory backend.** Section 3 with `memory()` only, under
   the `secure-store` feature, with no new dependencies. The tests are the
   same list as libmobile's:
   - names and sizes;
   - blob put, get and delete;
   - a key loads back with the same `public()`;
   - another kind gives `WrongKind`;
   - a malformed sealed form gives `Corrupt`;
   - a key and a blob with the same name do not meet;
   - a `compile_fail` doc test shows `SealToken(())` cannot be built
     outside the crate.
   **Done when:** `cargo test --features secure-store` and `cargo test`
   pass, and clippy is clean with and without the feature.
2. **The Keychain backend.** `keychain(service)` for both stores, with
   `security-framework` as an optional macOS dependency of the feature.
   The code moves from libmorph's `secure-store` branch. Two
   `#[ignore]`d tests (one per store) round-trip through the Keychain
   under the service `libdevice.test`.
   **Done when:** the owner runs `cargo test --features secure-store --
   --ignored keychain` from a Terminal and it passes.
3. **Example.** `examples/securestore.rs`, a command-line program with no
   window. Each run counts itself in blob `counter`, generates or loads a
   test key `device` (an example adapter: 32 random bytes from
   `/dev/urandom`, with a `u64` fingerprint as `Public`), and prints both.
   `--memory` uses the memory backend; `--reset` deletes both.
   **Done when:** two runs from a Terminal print counts 1 and 2 and the same
   fingerprint, and `security find-generic-password -s
   libdevice.securestore -a counter` finds the blob.

## 5. Release

Strictly additive: an optional feature, new modules and types. libdevice
0.1.0 becomes 0.1.1, which is semver-compatible, so libmorph takes it
without a Rule 5.5 bump of its own. This differs from the libmorph
opaque-keys amendment, which expected a minor bump. The commit is tagged
`v0.1.1`.

## 6. Open questions for the owner

1. **Matching libmobile's shapes.** libmobile's blob store is free
   functions (`android::secure_store::get(&app, ..)`), and its error has
   `Jni(String)`. For the same API in both crates, I propose two changes to
   libmobile before its release, while nothing is published: an
   `android::SecureStore` struct (`SecureStore::new(&app)` with the methods
   above, the functions kept or removed), and `Jni` renamed to `Platform`.
   Each would be an amendment to libmobile's secure-store plan.
2. **Versioning in libdevice.** libdevice has no tags or VERSIONS.md yet.
   Should it adopt libmorph's Rule 5 (VERSIONS.md, tags) with this release?

## Amendment — 2026-10-22 19:15 EDT: open questions settled

Settled with the owner (2026-10-22). Replaces section 6, and the sources
entry on plan rules.

1. **libmobile's shapes.** libmobile gets an `android::SecureStore`
   struct (`SecureStore::new(&app)` with this plan's blob methods) next to
   its functions, and its `SecureStoreError::Jni` becomes `Platform`
   ("Platform is a better name than Jni"). It is not released, so no
   existing error is renamed. This is done in libmobile and recorded there
   when it is implemented. libdevice has no Android code.
2. **Rules and versions.** libdevice now has an AGENTS.md with the critical
   rules (CHANGES.md, same date). This release follows them: 0.1.1, tagged
   `v0.1.1`, with the first `VERSIONS.md` entry in the version commit.

Also, when implementation starts, its first commit renames the error
variant `Locked` to `DeviceLocked` (the device's screen is locked) in
both error types of section 3, before anything else. `Locked` is kept for
keys that must be unlocked before use, which is a later plan. Owner: "Locked
and DeviceLocked are different state (like in Connectbot, where keys can be
unlocked to use them)", and "do the rename when we implement, before
anything else".
