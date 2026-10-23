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
