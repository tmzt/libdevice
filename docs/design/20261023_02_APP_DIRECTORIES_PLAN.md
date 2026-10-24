---
title: The app's data and cache directories on the desktop
date: 2026-10-23
status: In implementation
covers:
  - "`dirs::data_dir(app_id)` and `cache_dir(app_id)`: each desktop's
    per-user, per-app locations"
sources:
  - "Owner (2026-10-23, relayed by the handlr session): \"libmorph should
    provide a persistent storage location on all platforms, with
    mkdir_all\". This is the desktop half (libmorph AGENTS.md, Platform
    Layering), with the same shape as libmobile's
    docs/design/20261023_04_APP_DIRECTORIES_PLAN.md."
  - "handlr's request: macOS `~/Library/Application Support/<app id>`,
    Linux `$XDG_DATA_HOME/<app id>` (default `~/.local/share/<app id>`),
    Windows `%APPDATA%\\<app id>`, and matching cache directories."
  - "Apple's File System Programming Guide: Application Support for
    files the app creates and keeps, Caches for files it can recreate,
    both under the user's Library. XDG Base Directory Specification:
    `XDG_DATA_HOME` (default `~/.local/share`), `XDG_CACHE_HOME` (default
    `~/.cache`); relative values are ignored."
related:
  - libmobile docs/design/20261023_04_APP_DIRECTORIES_PLAN.md
  - libmorph docs/design/20261023_03_APP_STORAGE_PLAN.md (wraps both,
    and creates the directories)
---

# The app's data and cache directories on the desktop

## 1. API

Core, no feature and no dependencies. `src/dirs.rs`:

```rust
/// The per-user directory for files the app keeps:
/// macOS ~/Library/Application Support/<app_id>;
/// Linux and other Unix $XDG_DATA_HOME/<app_id> (default ~/.local/share);
/// Windows %APPDATA%\<app_id>. None if the home directory (or APPDATA)
/// is unknown, or `app_id` is not a valid name.
pub fn data_dir(app_id: &str) -> Option<PathBuf>;
/// The same for files the app can recreate: ~/Library/Caches/<app_id>;
/// $XDG_CACHE_HOME/<app_id> (default ~/.cache); %LOCALAPPDATA%\<app_id>\cache.
pub fn cache_dir(app_id: &str) -> Option<PathBuf>;
```

- Paths only: nothing is created (libmorph does that).
- `app_id` is 1 to 64 of `[A-Za-z0-9._-]`, not `.` or `..`, so it is
  always one path component. Anything else gives `None`.
- The home directory is `$HOME` on Unix and macOS, read once per call.
  Relative `XDG_*` values are ignored, as the specification says.
- The rules are a pure function of the platform and an environment
  lookup, tested on the host for every platform's rule with a fake
  environment.

## 2. Steps

1. **The functions** and their tests (each platform's rule, unset and
   relative `XDG_*`, a missing `HOME`, bad ids).
   **Done when:** `cargo test` and clippy pass, and an example,
   `examples/dirs.rs dev.tmzt.handlr`, prints
   `~/Library/Application Support/dev.tmzt.handlr` and
   `~/Library/Caches/dev.tmzt.handlr` (expanded) on the owner's Mac.
2. **Release** in the next libdevice release, on the owner's word.
   Additive.

## Amendment — 2026-10-23 23:32 EDT: approved

The owner approved the app storage plans in libmobile, libdevice and
libmorph on 2026-10-23 ("approved plan and start"). In libmorph's plan,
as amended ("`data_dir()` itself becomes cross-platform"), the answers
are: `data_dir()` keeps `Option<&Path>` ("it's missing on web so option
makes sense", relayed by the handlr session); a new `APP_ID`, with
`SECURE_STORE_ID` unchanged; the pipeline cache stays at
`<data_dir>/libmorph/`; `None` on the web for now; libdevice computes
the Linux and Windows paths too. The status stays `Draft` until the
first commit that implements it.

## Amendment — 2026-10-23 23:34 EDT: implementation has started; the document is now immutable

Step 1 starts with this commit.
