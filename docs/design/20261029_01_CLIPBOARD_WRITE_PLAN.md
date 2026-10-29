---
title: Writing text to the clipboard on the desktop
date: 2026-10-29
status: In implementation
covers:
  - "Feature `clipboard` gains `clipboard::write_text(text: &str) ->
    Result<(), ClipboardError>`: on macOS, the general pasteboard's
    `clearContents` then `setString:forType:` with
    `NSPasteboardTypeString` (`public.utf8-plain-text`)"
  - "`clipboard::ClipboardError` (`Unsupported`, `Platform(String)`);
    other desktops answer `Unsupported`, as the read answers `None`"
sources:
  - "Owner (2026-10-29, in the RoundTwo session, verbatim): \"also add a
    copy button (copy icon if available or Copy in 0.8 size) to the
    question and transcript in the review card\"."
  - "Owner (2026-10-29, in the RoundTwo session), asked how to proceed since
    no library writes the clipboard, chose: \"Plan it in the libraries:
    write a plan in libmobile for writing the clipboard (Android's
    ClipboardManager), then one in libmorph for ctx.set_clipboard_text
    (Android and macOS), each under its own rules and version. Then move
    RoundTwo's libmorph pin and add the buttons.\" libmorph's macOS half
    is built here first (libmorph AGENTS.md, Platform Layering)."
  - "`docs/design/20261024_01_CLIPBOARD_PLAN.md` (In implementation), 1:
    \"Read only: writing to the clipboard is not needed yet.\" That plan
    is frozen, so the write is this new plan (AGENTS.md Rule 1.7). Its
    step 1 is `31cc51a` (on `main`); its step 2, the release, is still
    open."
  - "libdevice today (0.1.2, `31cc51a`): `src/clipboard.rs` with
    `read_text() -> Option<String>`, macOS through
    `NSPasteboard::generalPasteboard().stringForType(NSPasteboardTypeString)`
    inside `autoreleasepool`, `None` on other desktops;
    `examples/clipboard.rs`; objc2 0.5, objc2-foundation 0.2 (`NSString`,
    `NSURL`) and objc2-app-kit 0.2 (`NSPasteboard`, `NSWorkspace`)."
  - "macOS: `-[NSPasteboard clearContents]` (returns the new change count;
    required before writing, since it makes the app the pasteboard's
    owner) and `-[NSPasteboard setString:forType:]` (returns `BOOL`);
    `NSPasteboardTypeString` is `public.utf8-plain-text`. macOS shows no
    confirmation of a copy."
related:
  - docs/design/20261024_01_CLIPBOARD_PLAN.md (the read, extended here in
    the same feature and module)
  - libmobile docs/design/20261029_01_CLIPBOARD_WRITE_PLAN.md (the Android
    write)
  - libmobile docs/design/20261028_03_CLIPBOARD_PLAN.md (the Android read)
  - libmorph docs/design/20261029_01_CLIPBOARD_WRITE_PLAN.md (wraps this
    and libmobile's as `MorphContext::set_clipboard_text`)
  - libmorph docs/design/20261028_03_CLIPBOARD_PLAN.md (the read on every
    host)
  - RoundTwo docs/design/20261029_06_MOCK_INTERVIEW_SESSION_PLAN.md (the
    first user: Copy on the results' question text and each answer's
    transcript)
---

# Writing text to the clipboard on the desktop

## 1. API

The existing feature `clipboard`, off by default, gains the write in
`src/clipboard.rs`. A new feature is not needed: the write uses the same
three crates and the same `NSPasteboard` binding, and an app that enables
`clipboard` today is unaffected.

```rust
/// Why the clipboard could not be written.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClipboardError {
    /// No clipboard call on this desktop (anything but macOS, for now).
    Unsupported,
    /// The platform refused or failed; on macOS, `setString:forType:`
    /// returned NO.
    Platform(String),
}
// Display and std::error::Error.

/// Put `text` on the clipboard as plain text, replacing what was there.
/// macOS: the general pasteboard is cleared, then given `text` as
/// `NSPasteboardTypeString` (`public.utf8-plain-text`). Other desktops:
/// `Err(ClipboardError::Unsupported)`, until they are supported.
///
/// An empty `text` is written as given. Any thread; the macOS call runs
/// inside its own autorelease pool, as `read_text` does. Nothing is
/// logged, neither the text nor its length. macOS shows no confirmation of
/// a copy; the app shows its own if it wants one.
pub fn write_text(text: &str) -> Result<(), ClipboardError>;
```

`read_text` is unchanged. The write returns a `Result` where the read
returns an `Option`, because a Copy button must know whether it worked:
"nothing to read" is a normal answer, "could not write" is not. The shape
of `ClipboardError` follows `link::OpenLinkError` (`Unsupported`,
`Platform`), and the call's name and argument follow libmobile's
`android::clipboard::write_text` without the `AndroidApp`.

### The macOS body

```rust
pub(super) fn write_text(text: &str) -> Result<(), ClipboardError> {
    autoreleasepool(|_| {
        let string = NSString::from_str(text);
        // SAFETY: `generalPasteboard`, `clearContents` and
        // `setString:forType:` may be called from any thread, and
        // `NSPasteboardTypeString` is a constant NSString AppKit defines.
        let written = unsafe {
            let pasteboard = NSPasteboard::generalPasteboard();
            pasteboard.clearContents();
            pasteboard.setString_forType(&string, NSPasteboardTypeString)
        };
        if written {
            Ok(())
        } else {
            Err(ClipboardError::Platform("the pasteboard refused the text".into()))
        }
    })
}
```

`clearContents` comes first, as AppKit requires before writing new
contents (without it `setString:forType:` fails once another app owns the
pasteboard). Its returned change count is not needed. objc2-foundation's
`NSString` feature, already on, gives `NSString::from_str`; no new crate
or crate feature is needed (if objc2-app-kit 0.2 gates `clearContents` or
`setString_forType` behind another feature, that feature is added to the
existing dependency line and recorded as a departure).

Other desktops: `Err(ClipboardError::Unsupported)`, as the read plan
answers `None` there.

## 2. Files

- `src/clipboard.rs`: `ClipboardError` with `Display` and `Error`,
  `write_text`, the macOS and other-desktop bodies, module docs ("read and
  write"), tests: on other desktops `write_text("a")` is `Unsupported`;
  `Display` of both variants.
- `Cargo.toml`: the `clipboard` feature's comment.
- `examples/clipboard.rs`: with no argument it prints the read's length as
  today; with `--write <text>` it calls `write_text` and prints `ok` or the
  error (not the text).
- `README.md`: the feature's line gains the write (release step).

## 3. Versioning

Additive: a new function and a new `#[non_exhaustive]` error type in an
existing feature. The clipboard read (`31cc51a`) is not released yet, so
both land together in the next release, libdevice 0.1.2 to **0.1.3**,
tagged `v0.1.3`, with one `VERSIONS.md` entry naming both plans. This also
completes the read plan's step 2 (its own amendment records it then).
libmorph re-exports libdevice (`pub use libdevice;`); the change is
additive there too.

## 4. Steps

1. **The call**: `ClipboardError`, `write_text`, the tests and the
   example's `--write`.
   **Done when:** `cargo fmt -p libdevice -- --check`; `cargo clippy -p
   libdevice --all-targets` with no features and with `--features
   clipboard`: no new warnings; `cargo test -p libdevice` and `cargo test
   -p libdevice --features clipboard` pass; on the owner's Mac this script
   exits 0:

   ```sh
   export LANG=en_US.UTF-8
   cargo run -q -p libdevice --features clipboard --example clipboard -- --write 'héllo ✓' | grep -qx ok
   test "$(pbpaste)" = 'héllo ✓'
   echo hello | pbcopy
   cargo run -q -p libdevice --features clipboard --example clipboard -- --write 'second'
   test "$(pbpaste)" = second
   ```

   (a non-ASCII text round-trips, and a write replaces what another app
   put there).
2. **Release** as libdevice 0.1.3, with the clipboard read, on the
   owner's word: the README lines, `Cargo.toml`'s version, the
   `VERSIONS.md` entry, the tag `v0.1.3`.
   **Done when:** the commit is tagged `v0.1.3` and libmorph builds on
   macOS against it with `write_text` called from its clipboard module.

## 5. Open questions

1. **Concealed text.** Should a later option mark the text with the
   nspasteboard.org `org.nspasteboard.ConcealedType` (and
   `TransientType`), so clipboard managers do not keep it? It pairs with
   libmobile's open question on `EXTRA_IS_SENSITIVE`. Proposed: not now;
   interview text is not secret. It would be an additive `write_text_with`.
2. **Other desktops.** Linux (X11 selections need a process that stays
   alive to serve them; Wayland needs a seat) and Windows stay
   `Unsupported` here, as the read plan leaves them `None`. Is that right
   for now? Proposed: yes; a later plan when an app ships there.

## 6. Risks

- **Threads.** `NSPasteboard` is used off the main thread by the read
  already; Apple documents no main-thread requirement for it. If the
  write misbehaves off the main thread, libmorph calls it from its event
  loop's thread (the main thread on macOS) anyway.

## Amendment — 2026-10-29 17:34 EDT: approved; implementation started

The owner approved this plan on 2026-10-29, with the other clipboard write
plans (libmobile's and libmorph's): "approve all four plans and build
them", and "push them when built". The open questions take the proposed
answers: no concealed or transient type for now (section 5, question 1),
and Linux and Windows stay `Unsupported` (section 5, question 2).

Implementation has started with this commit (step 1); the document is now
immutable.
