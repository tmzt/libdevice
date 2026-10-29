//! **Clipboard** (feature `clipboard`): read the clipboard's text, for
//! pasting, and write text to it, for copying.
//!
//! * macOS: the general pasteboard's plain-text string
//!   (`NSPasteboardTypeString`, `public.utf8-plain-text`), from any
//!   thread. A write clears the pasteboard first.
//! * Other desktops: the read is `None` and the write
//!   [`ClipboardError::Unsupported`], until they are supported.
//!
//! Nothing is logged.

use std::fmt;

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

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => {
                f.write_str("writing the clipboard is not supported on this platform")
            }
            Self::Platform(message) => write!(f, "writing the clipboard failed: {message}"),
        }
    }
}

impl std::error::Error for ClipboardError {}

/// The clipboard's text, if it holds any. macOS: the general pasteboard's
/// plain-text string. Other desktops: None, until they are supported.
///
/// Any thread; the macOS call runs inside its own autorelease pool.
#[must_use]
pub fn read_text() -> Option<String> {
    platform::read_text()
}

/// Put `text` on the clipboard as plain text, replacing what was there.
/// macOS: the general pasteboard is cleared, then given `text` as
/// `NSPasteboardTypeString` (`public.utf8-plain-text`). Other desktops:
/// `Err(ClipboardError::Unsupported)`, until they are supported.
///
/// An empty `text` is written as given. Any thread; the macOS call runs
/// inside its own autorelease pool, as [`read_text`] does. Nothing is
/// logged, neither the text nor its length. macOS shows no confirmation of
/// a copy; the app shows its own if it wants one.
pub fn write_text(text: &str) -> Result<(), ClipboardError> {
    platform::write_text(text)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::ClipboardError;
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;

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

    pub(super) fn write_text(text: &str) -> Result<(), ClipboardError> {
        autoreleasepool(|_| {
            let string = NSString::from_str(text);
            // SAFETY: `generalPasteboard`, `clearContents` and
            // `setString:forType:` may be called from any thread, and
            // `NSPasteboardTypeString` is a constant NSString AppKit
            // defines. `clearContents` comes first: it makes this app the
            // pasteboard's owner, which writing requires.
            let written = unsafe {
                let pasteboard = NSPasteboard::generalPasteboard();
                pasteboard.clearContents();
                pasteboard.setString_forType(&string, NSPasteboardTypeString)
            };
            if written {
                Ok(())
            } else {
                Err(ClipboardError::Platform(
                    "the pasteboard refused the text".into(),
                ))
            }
        })
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::ClipboardError;

    pub(super) fn read_text() -> Option<String> {
        None
    }

    pub(super) fn write_text(_text: &str) -> Result<(), ClipboardError> {
        Err(ClipboardError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::ClipboardError;

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn other_desktops_have_no_clipboard() {
        assert_eq!(super::read_text(), None);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn other_desktops_cannot_write() {
        assert_eq!(super::write_text("a"), Err(ClipboardError::Unsupported));
    }

    #[test]
    fn errors_display() {
        assert_eq!(
            ClipboardError::Unsupported.to_string(),
            "writing the clipboard is not supported on this platform"
        );
        assert_eq!(
            ClipboardError::Platform("the pasteboard refused the text".into()).to_string(),
            "writing the clipboard failed: the pasteboard refused the text"
        );
    }
}
