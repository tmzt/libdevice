---
title: Disk space and disk pressure on the desktop, from one engine file shared with libmobile
date: 2026-11-19
status: In implementation
covers:
  - "Feature `disk-space`: `disk_space::space(path)` on macOS, Linux and other Unix through statvfs, `Unsupported` on Windows and the web"
  - "The disk-space engine, `src/disk_space/engine.rs`: `DiskSpace`, `DiskPressure`, `DiskThresholds::grade`, `DiskPressureReport`, `DiskSpaceError`, `statvfs_space` and the check-interval constants, byte-identical in libmobile and libdevice, with an `ENGINE_SOURCE` check in libmorph"
sources:
  - "Owner (2026-11-19, through the ShotToss coordinator), on libmorph's disk pressure plan's open question 1: \"just have the subagent do it and follow the critical rules, must be tested and share API with libmorph\""
  - "libmobile `docs/design/20261119_02_DISK_SPACE_PLAN.md` (Draft): the types, the grade with its thresholds and hysteresis, `statvfs_space`, `DISK_CHECK_INTERVAL` (60 s, bounds 15 s to 10 min)"
  - "libmorph `docs/design/20261119_02_DISK_PRESSURE_PLAN.md` (Draft): `MorphApp::DISK_WATCH`, `on_disk_pressure`, `ctx.disk_space`, the desktop's own timer over libdevice"
  - "libmorph `docs/design/20261022_06_KEY_STORE_PLAN.md` (Implemented): one small engine file byte-identical in libmobile and libdevice, each ending with `ENGINE_SOURCE = include_str!(..)`, compared by a libmorph test (`tests/secure_store.rs`)"
  - "libdevice today (0.1.3, `948f924`): no disk call; desktops it already distinguishes are macOS, Linux and other Unix, and Windows (`dirs`)"
related:
  - libmobile docs/design/20261119_02_DISK_SPACE_PLAN.md (the Android half and the engine's first home)
  - libmorph docs/design/20261119_02_DISK_PRESSURE_PLAN.md (wraps both)
  - docs/design/20261023_02_APP_DIRECTORIES_PLAN.md (the directories whose volume libmorph watches)
---

# Disk space and disk pressure on the desktop, from one engine file shared with libmobile

## 1. What it is

The desktop half of disk pressure: how much room the volume holding a path
has, and the same grading libmobile uses, so libmorph's `on_disk_pressure`
behaves the same on a Mac as on a phone. The checking timer is libmorph's
(its plan), not libdevice's: libdevice gives the numbers and the grade.

## 2. One engine file, and how its identity is checked

The types and the grading are written once, in `src/disk_space/engine.rs`,
and the file is **byte-identical** in libmobile and libdevice, as the key
store engine is. Each crate's `src/disk_space.rs` is its own:

| Path | libdevice | libmobile |
|---|---|---|
| `src/disk_space/engine.rs` | the engine, identical | the engine, identical |
| `src/disk_space.rs` | `mod engine; pub use engine::*;` and `space(path)` (3) | `mod engine; pub use engine::*;` (its Android `space` stays in `android::disk_space`) |

The engine ends with:

```rust
#[doc(hidden)]
pub const ENGINE_SOURCE: &str = include_str!("engine.rs");
```

**Identity is checked in libmorph**, which depends on both: a test built
on the host with `--features mobile,mobile-disk-space` (libmobile's
platform-free modules build there) asserts
`libmobile::disk_space::ENGINE_SOURCE == libdevice::disk_space::ENGINE_SOURCE`,
beside the key store's. libdevice's release step also compares the file
with libmobile's at libmobile's release tag (`cmp`), so a drift is caught
before either release, not only when libmorph next builds.

**Recommendation: the identical file, not a shared crate.** The engine is
about 200 lines with its tests and has no dependency but `libc`. A shared
crate would be a fourth repository with its own versions and tags, and
under Rule 5.5 every incompatible change in it would ripple through
libmobile, libdevice, libmorph and libmorph-build as minor bumps; the file
copy has one owner (libmobile first, copied to libdevice in the same
release cycle) and an automatic check, and has worked for the key store.
If a third engine appears, a shared `libplatform-core` crate would be
worth its own plan.

## 3. API

The engine (both crates), exactly libmobile's types:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] #[non_exhaustive]
pub struct DiskSpace { pub free: u64, pub total: u64, pub allocatable: Option<u64>, pub cache_quota: Option<u64> }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)] #[non_exhaustive]
pub enum DiskPressure { None, Low, Critical }

#[derive(Clone, Copy, Debug, PartialEq, Eq)] #[non_exhaustive]
pub struct DiskThresholds { pub low_min: u64, pub low_fraction_permille: u32, pub critical: u64, pub hysteresis: u64 }
impl DiskThresholds {
    pub const DEFAULT: Self; // 1 GiB, 50 permille, 256 MiB, 256 MiB
    pub const fn new(low_min: u64, low_fraction_permille: u32, critical: u64, hysteresis: u64) -> Self;
    pub fn low_threshold(&self, total: u64) -> u64; // max(low_min, total * permille / 1000), saturating
    pub fn grade(&self, prev: DiskPressure, free: u64, total: u64) -> DiskPressure;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)] #[non_exhaustive]
pub struct DiskPressureReport { pub pressure: DiskPressure, pub space: DiskSpace }
impl DiskPressureReport { pub const fn new(pressure: DiskPressure, space: DiskSpace) -> Self; }

#[derive(Clone, Debug, PartialEq, Eq)] #[non_exhaustive]
pub enum DiskSpaceError { Path(String), Unsupported, Platform(String) }
// Display, std::error::Error

/// statvfs(path): free = f_bavail * f_frsize, total = f_blocks * f_frsize,
/// saturating; allocatable and cache_quota None. cfg(unix); elsewhere
/// Err(Unsupported).
pub fn statvfs_space(path: &Path) -> Result<DiskSpace, DiskSpaceError>;

pub const DISK_CHECK_INTERVAL: Duration;     // 60 s
pub const DISK_CHECK_INTERVAL_MIN: Duration; // 15 s
pub const DISK_CHECK_INTERVAL_MAX: Duration; // 600 s
pub fn clamp_interval(interval: Duration) -> Duration;
```

`new` and `DiskPressureReport::new` exist because the structs are
`#[non_exhaustive]`, so libmorph and apps outside the crate can build them
(libmorph's scripted runs do). `DiskSpaceError::Unsupported` covers
non-Unix hosts; on Android it never occurs.

**The grade**, as libmobile's plan states it: rising is immediate (free
under `critical` gives `Critical`, else under `low_threshold` gives `Low`);
falling needs free at least threshold + `hysteresis` (from `Critical`, at
or above `critical + hysteresis` gives `Low`, and if also at or above
`low_threshold + hysteresis`, `None`; from `Low`, at or above
`low_threshold + hysteresis` gives `None`).

libdevice's own, in `src/disk_space.rs` (feature `disk-space`):

```rust
/// The space of the volume holding `path` on this desktop: statvfs on
/// macOS, Linux and other Unix; Err(Unsupported) on Windows and wasm32.
/// Any thread, microseconds.
pub fn space(path: &Path) -> Result<DiskSpace, DiskSpaceError>;
```

Windows (`GetDiskFreeSpaceExW`) is left until a Windows app asks.

## 4. Features and dependencies

- `disk-space = ["dep:libc"]`, with a comment, after `clipboard`.
- `[target.'cfg(unix)'.dependencies] libc = { version = "0.2", optional = true }`.
  The engine's `statvfs_space` is under `#[cfg(unix)]`, so on Windows and
  wasm32 the feature builds without `libc`; there the optional dependency
  is simply not resolved for the target.

## 5. Tests

All in the engine file (so identical in both crates), under
`#[cfg(test)]`, except the platform `space` test:

- **Space on a temp dir:** `statvfs_space(&std::env::temp_dir())` gives
  `total > 0`, `free <= total`, both `None`s; a missing path gives `Path`;
  the saturating multiply at `u64::MAX`. In libdevice, `space` on the temp
  dir equals `statvfs_space` there, on Unix, and is `Unsupported` on
  Windows.
- **The grading table**, one row per case, `DEFAULT` on a 64 GiB volume
  (Low threshold 3.2 GiB from the 5 % term) and on a 16 GiB one (1 GiB
  from the floor): each rise from `None`; `Critical` straight from `None`;
  at exactly a threshold (not below: unchanged); just under; falling from
  `Low` 1 byte short of threshold + hysteresis (stays `Low`) and at it
  (`None`); from `Critical` to `Low` and from `Critical` straight to `None`
  on a large recovery; a volume oscillating ±100 MiB around the Low
  threshold giving one change, not one per sample; `low_threshold`
  saturating.
- **Intervals:** `clamp_interval` of 1 s, 60 s and 1 h.
- **The identical file:** in libmorph (2), and `cmp` at release here.

## 6. Versioning

Additive: a new feature, a new module and `#[non_exhaustive]` types.
libdevice 0.1.3 to **0.1.4**, tagged `v0.1.4`, with a `VERSIONS.md` entry.
libmorph takes it without a minor bump.

## 7. Steps

1. **The engine and `space`**, after libmobile's engine file exists (the
   libmobile plan's step 1, amended to put it at `src/disk_space/engine.rs`):
   copy the engine byte for byte, add `src/disk_space.rs` with `space`, the
   feature, the dependency and `lib.rs`'s module.
   **Done when:** `cargo fmt -- --check`; `cargo clippy --all-targets`
   with no features and `--all-features`: no warnings; `cargo test
   --all-features` passes on the Mac, including the temp-dir space tests,
   every row of the grading table and the interval clamp; `cargo check
   --features disk-space --target x86_64-pc-windows-gnu` (or the CI's
   Windows target) builds without `libc`; `cmp src/disk_space/engine.rs
   ~/src/libmobile/src/disk_space/engine.rs` reports no difference; on the
   Mac, `space` of the home directory matches `df -k ~` within one block.
2. **Release** as libdevice 0.1.4 on the owner's word: README line,
   version, `VERSIONS.md` entry, tag; the `cmp` against libmobile's file at
   its release tag repeated.
   **Done when:** tagged `v0.1.4`, and libmorph's engine-equality test
   (`libmobile::disk_space::ENGINE_SOURCE ==
   libdevice::disk_space::ENGINE_SOURCE`) passes with both releases.

## 8. Risks

- **Two copies drift.** The libmorph test fails the moment they differ;
  the rule is libmobile first, copied here in the same cycle.
- **APFS purgeable space.** On macOS, `statvfs` free excludes purgeable
  space that Finder counts as available, so the Mac may grade lower than
  Finder suggests. Conservative for a cache; noted in `space`'s docs.

## Amendment — 2026-11-19 18:36 EST: the low-level call only; no grading and no shared engine file

The owner decided on 2026-11-19, through the ShotToss coordinator
(verbatim): "we can provide the low-level API in libmobile/libdevice and
the high-level grading in libmorph, this doesn't have the security
considerations of keystore (which is what the source-level checks are
for)". That answer is the agreement for this amendment. This plan has no
earlier amendment. It replaces, in the original text:

- **Section 2 entirely** ("One engine file, and how its identity is
  checked", with its recommendation): there is no
  `src/disk_space/engine.rs`, no `ENGINE_SOURCE`, no libmorph equality
  test and no `cmp` against libmobile. libdevice's types are its own.
- **Section 3's engine block** by libdevice's own, in `src/disk_space.rs`
  (feature `disk-space`):

  ```rust
  /// A volume's space, in bytes, from statvfs.
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  #[non_exhaustive]
  pub struct DiskSpace { pub free: u64, pub total: u64 }

  #[derive(Clone, Debug, PartialEq, Eq)]
  #[non_exhaustive]
  pub enum DiskSpaceError { Path(String), Unsupported, Platform(String) }

  /// free = f_bavail * f_frsize, total = f_blocks * f_frsize, saturating.
  /// macOS, Linux and other Unix; Err(Unsupported) on Windows and wasm32.
  /// Any thread, microseconds.
  pub fn space(path: &Path) -> Result<DiskSpace, DiskSpaceError>;
  ```

  `DiskSpace` has no `allocatable` or `cache_quota`: the desktop has
  neither. `DiskPressure`, `DiskThresholds`, `grade`,
  `DiskPressureReport`, `statvfs_space` as a separate name, the interval
  constants and `clamp_interval` are removed; grading, thresholds and the
  schedule are libmorph's (its disk pressure plan, amended the same day),
  and libmorph converts this `DiskSpace` into its own type. "The grade"
  paragraph is removed.
- **Section 4**: `libc` stays a `cfg(unix)` optional dependency of
  `disk-space`; the text about the engine is removed.
- **Section 5** by: `space(&std::env::temp_dir())` gives `total > 0` and
  `free <= total` on Unix and `Unsupported` elsewhere; a missing path is
  `Path`; the saturating multiply at `u64::MAX`. The grading table, the
  interval clamp and the identical-file check are removed.
- **Step 1** by: **the call.** `src/disk_space.rs`, the feature, the
  dependency, `lib.rs`'s module. **Done when:** `cargo fmt -- --check`;
  `cargo clippy --all-targets` with no features and `--all-features`: no
  warnings; `cargo test --all-features` passes on the Mac with the tests
  above; `cargo check --features disk-space --target
  x86_64-pc-windows-gnu` builds without `libc`; on the Mac, `space` of
  the home directory matches `df -k ~` within one block. It no longer
  waits for libmobile.
- **Step 2's Done when** by: tagged `v0.1.4`, and libmorph's desktop
  `disk_space` builds and passes its tests against it.
- **Section 8's first risk** ("Two copies drift") is removed; the APFS
  purgeable-space risk stays.
- **The title's "from one engine file shared with libmobile"** no longer
  holds; the plan gives the desktop's free and total space.

Versioning stands: additive, 0.1.3 to 0.1.4.

## Amendment — 2026-11-19 18:58 EST: implementation started; the document is now immutable

The owner approved this plan, as amended, on 2026-11-19 ("approve and
build"). Implementation starts with step 1 in the commit that adds this
amendment; `status` is now `In implementation`, and the document is
immutable from here on, changed only on the owner's explicit instruction.
