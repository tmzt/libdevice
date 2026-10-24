---
title: Opening a link in another app on the desktop
date: 2026-10-23
status: Draft
covers:
  - "Feature `open-link`: `open_link(url)` with libmobile's API, on macOS
    through `NSWorkspace`"
  - Other desktops report it as unsupported
sources:
  - "handlr (request, 2026-10-23): an outbound \"open this link\" call for
    repowiki links in its agent view; on the desktop, the system opener.
    For the owner's approval in the libmorph session."
  - "libmorph AGENTS.md, Platform Layering: a desktop platform capability
    is built in libdevice first, behind a feature, with the same API as
    libmobile's, and libmorph wraps both."
  - "libmobile docs/design/20261023_03_OPEN_LINK_PLAN.md: `check_link`,
    `open_link` and `OpenLinkError`."
  - "macOS: `NSWorkspace.sharedWorkspace.openURL:` (returns `NO` when no
    app opens it); objc2 0.5 and objc2-app-kit 0.2, which libmorph already
    uses for the trackpad."
related:
  - libmobile docs/design/20261023_03_OPEN_LINK_PLAN.md
  - libmorph docs/design/20261023_02_OPEN_LINK_PLAN.md (wraps both)
  - docs/design/20261022_01_SECURE_STORE_AND_KEY_STORE_PLAN.md (the same
    pattern: libmobile's API, behind a feature)
---

# Opening a link in another app on the desktop

## 1. Goal

The same call as libmobile's, so libmorph can wrap both: the app asks the
system to open an http or https link, and the default browser opens it.

## 2. API

Feature `open-link`, off by default. `src/link.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenLinkError {
    BadUrl,
    NoHandler,
    /// No system opener on this platform (desktops other than macOS).
    Unsupported,
    Platform(String),
}

pub fn check_link(url: &str) -> Result<(), OpenLinkError>;
pub fn open_link(url: &str) -> Result<(), OpenLinkError>;
```

- `check_link` is the same rule as libmobile's (http and https, a host,
  no whitespace or control characters), with the same tests. The two are
  kept equal by a libmorph test, as the key store engine is.
- **macOS:** `NSURL::URLWithString`, then
  `NSWorkspace::sharedWorkspace().openURL(&url)`; `NO` is `NoHandler`.
  Any thread (`NSWorkspace` is thread-safe for this call). Dependencies
  under the feature, macOS only: `objc2` 0.5 and `objc2-app-kit` 0.2 with
  `NSWorkspace`, and `objc2-foundation` 0.2 with `NSURL`.
- **Other desktops:** `Unsupported`, until Linux or Windows is supported.
- Nothing is logged.

`OpenLinkError` here has `Unsupported`, which libmobile's has not, and no
`NotVisible`; libmorph maps both into its own error.

## 3. Steps

1. **The call.** The feature, `check_link` with its tests, and
   `open_link`.
   **Done when:** `cargo test` (with and without the feature) and clippy
   pass, and an example, `examples/open_link.rs https://example.com/`,
   opens the default browser on the owner's Mac.
2. **Release** in the next libdevice release, on the owner's word.
   Additive: a new feature, module, type and functions.

## 4. Open question for the owner

1. **`NSWorkspace` or `/usr/bin/open`.** Running `open <url>` needs no
   new dependencies, but its failure is only an exit status and it starts
   a process per link. Recommended: `NSWorkspace`, with the objc2 crates
   libmorph already builds.
