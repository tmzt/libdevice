---
title: Reading text from the clipboard on the desktop
date: 2026-10-24
status: In implementation
covers:
  - "Feature `clipboard`: `clipboard::read_text() -> Option<String>`, on
    macOS through `NSPasteboard`"
sources:
  - "handlr (request, 2026-10-24, with the owner's \"yes\"): text input on
    the desktop with paste handled, so one code path serves the phone and
    the Mac. For the owner's approval in the libmorph session."
  - "libmorph AGENTS.md, Platform Layering: OS services on the desktop
    are built in libdevice first, behind a feature."
  - "macOS: `NSPasteboard.generalPasteboard.stringForType:
    NSPasteboardTypeString`; objc2 0.5 and objc2-app-kit 0.2, already
    used by libdevice's `open-link`."
related:
  - libmorph docs/design/20261024_06_TEXT_INPUT_PLAN.md (pastes with it)
---

# Reading text from the clipboard on the desktop

## 1. API

Feature `clipboard`, off by default. `src/clipboard.rs`:

```rust
/// The clipboard's text, if it holds any. macOS: the general pasteboard's
/// plain-text string. Other desktops: None, until they are supported.
pub fn read_text() -> Option<String>;
```

Read only: writing to the clipboard is not needed yet. Nothing is
logged. Any thread, inside its own autorelease pool, as `open_link` is.

## 2. Steps

1. **The call**, with `examples/clipboard.rs` printing the text's length.
   **Done when:** clippy and tests pass with and without the feature, and
   on the owner's Mac, after `echo hello | pbcopy`, the example prints 6.
2. **Release** in the next libdevice release, on the owner's word.
   Additive.

## Amendment — 2026-10-24 16:18 EDT: approved

The owner approved the text input plans in libmobile, libdevice and
libmorph on 2026-10-24 ("yes"), with libmorph's recommended answers: on
the desktop, text is edited at its end only (no caret movement or
selection); paste through libdevice's `clipboard`; `unicode-segmentation`
for Backspace. They are built after the notification and dark mode work
and released after it (libmobile 0.2.7, the next libdevice release,
libmorph 0.4.4), so they do not hold it up. The status stays `Draft`
until the first commit that implements it.

## Amendment — 2026-10-24 16:19 EDT: implementation has started; the document is now immutable

Step 1 starts with this commit.
