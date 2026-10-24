//! **Clipboard** (feature `clipboard`): read the clipboard's text, for
//! pasting. Read only.
//!
//! * macOS: the general pasteboard's plain-text string
//!   (`NSPasteboard.generalPasteboard.stringForType:
//!   NSPasteboardTypeString`), from any thread.
//! * Other desktops: `None`, until they are supported.
//!
//! Nothing is logged.

/// The clipboard's text, if it holds any. macOS: the general pasteboard's
/// plain-text string. Other desktops: None, until they are supported.
///
/// Any thread; the macOS call runs inside its own autorelease pool.
#[must_use]
pub fn read_text() -> Option<String> {
    platform::read_text()
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};

    pub(super) fn read_text() -> Option<String> {
        // A pool of its own, so a call from a thread without one does not
        // leak the autoreleased objects.
        autoreleasepool(|pool| {
            // SAFETY: `generalPasteboard` and `stringForType:` may be
            // called from any thread, and `NSPasteboardTypeString` is a
            // constant NSString that AppKit defines.
            let text =
                unsafe { NSPasteboard::generalPasteboard().stringForType(NSPasteboardTypeString) }?;
            // Copied out before the pool drains.
            Some(text.as_str(pool).to_owned())
        })
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub(super) fn read_text() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn other_desktops_have_no_clipboard() {
        assert_eq!(super::read_text(), None);
    }
}
