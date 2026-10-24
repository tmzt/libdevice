# Versions

Each version change, oldest first, under the critical rules in AGENTS.md
(rule 5). 0.1.0 predates the rules and has no entry or tag.

## 0.1.1 — 2026-10-22 (tag `v0.1.1`)

The desktop secure store and key store
(`docs/design/20261022_01_SECURE_STORE_AND_KEY_STORE_PLAN.md`, and libmorph
`docs/design/20261022_06_KEY_STORE_PLAN.md`). All additions, behind
features that are off by default. The commit is the one tagged `v0.1.1`.

- **Feature `secure-store`:**
  - the module `secure_store`: `SecureStore` (`keychain(service)` on
    macOS, `memory()`, `get`, `put`, `put_with`, `delete`,
    `security_level`), with `PutOptions`, `SecureStoreError` and
    `SecurityLevel`;
  - the module `key_store`, the same engine file as libmobile's:
    `generate`, `public`, `with_key` and `delete` over a `Vault`
    (`Vault::named(namespace)` on macOS, in the login Keychain), with
    `SecretKey`, `SealToken`, `KeyStoreError` and the flags;
  - optional dependencies `zeroize` 1.8 and `log` 0.4, and on macOS
    `security-framework` 3.
- **Feature `test-vault`** (implies `secure-store`): `Vault::memory()`,
  for tests only.

## 0.1.2 — 2026-10-23 (tag `v0.1.2`)

Opening links (`docs/design/20261023_01_OPEN_LINK_PLAN.md`) and the app's
directories (`docs/design/20261023_02_APP_DIRECTORIES_PLAN.md`). All
additions. The commit is the one tagged `v0.1.2`.

- **Feature `open-link`** (off by default): the module `link`, with
  `check_link(url)`, `open_link(url)` (macOS through `NSWorkspace`;
  `Unsupported` on other desktops) and `OpenLinkError` (`BadUrl`,
  `NoHandler`, `Unsupported`, `Platform`), which implements `Display` and
  `Error`. On macOS it adds `objc2` 0.5, `objc2-foundation` 0.2 and
  `objc2-app-kit` 0.2.
- **The module `dirs`** (core, no dependencies): `data_dir(app_id)` and
  `cache_dir(app_id)`, the per-user, per-app paths on macOS, Linux and
  other Unix (XDG) and Windows; `None` elsewhere or for a bad id.
