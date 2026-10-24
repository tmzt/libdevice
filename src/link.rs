//! **Open link** (feature `open-link`): ask the system to open an http or
//! https link in the app that handles it, normally the default browser.
//! The same call as libmobile's on Android, so libmorph only wraps it.
//!
//! * macOS: `NSWorkspace.sharedWorkspace.openURL:`, from any thread.
//! * Other desktops: [`OpenLinkError::Unsupported`].
//!
//! Nothing is logged.

use std::fmt;

/// Why a link was not opened.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenLinkError {
    /// Not an absolute http or https URL with a host, or it holds
    /// whitespace or control characters.
    BadUrl,
    /// No app on the system opens it.
    NoHandler,
    /// No system opener on this platform (desktops other than macOS).
    Unsupported,
    /// The platform failed.
    Platform(String),
}

impl fmt::Display for OpenLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadUrl => f.write_str("not an http or https link"),
            Self::NoHandler => f.write_str("no app opens this link"),
            Self::Unsupported => f.write_str("opening links is not supported on this platform"),
            Self::Platform(message) => write!(f, "opening the link failed: {message}"),
        }
    }
}

impl std::error::Error for OpenLinkError {}

/// The check [`open_link`] makes before handing the link to the system:
/// the scheme is `http` or `https` (any case), followed by `://` and a
/// non-empty host, and the link holds no whitespace or control characters.
/// Every other scheme (`file:`, `content:`, `intent:`, `javascript:`,
/// `mailto:` and the rest) is [`OpenLinkError::BadUrl`]. Platform-free.
pub fn check_link(url: &str) -> Result<(), OpenLinkError> {
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(OpenLinkError::BadUrl);
    }
    let (scheme, rest) = url.split_once("://").ok_or(OpenLinkError::BadUrl)?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err(OpenLinkError::BadUrl);
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host_and_port = authority.rsplit('@').next().unwrap_or_default();
    let host = match host_and_port.strip_prefix('[') {
        // An IPv6 literal: everything up to the closing bracket.
        Some(literal) => literal.split_once(']').ok_or(OpenLinkError::BadUrl)?.0,
        None => host_and_port.split(':').next().unwrap_or_default(),
    };
    if host.is_empty() {
        return Err(OpenLinkError::BadUrl);
    }
    Ok(())
}

/// Open `url` (http or https) in the app that handles it, normally the
/// default browser. Any thread. Returns once the system has taken the
/// link; it does not wait for the browser.
///
/// Fails with [`OpenLinkError::BadUrl`] when [`check_link`] does,
/// [`OpenLinkError::NoHandler`] when no app opens it, and
/// [`OpenLinkError::Unsupported`] on desktops other than macOS.
pub fn open_link(url: &str) -> Result<(), OpenLinkError> {
    check_link(url)?;
    platform::open(url)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::OpenLinkError;
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};

    pub(super) fn open(url: &str) -> Result<(), OpenLinkError> {
        // A pool of its own, so a call from a thread without one does not
        // leak the autoreleased objects.
        autoreleasepool(|_| {
            let string = NSString::from_str(url);
            // SAFETY: `URLWithString:` takes any string and returns nil
            // when it is not a valid URL.
            let url = unsafe { NSURL::URLWithString(&string) }.ok_or(OpenLinkError::BadUrl)?;
            // SAFETY: `sharedWorkspace` and `openURL:` may be called from
            // any thread; `url` is a valid NSURL.
            let opened = unsafe { NSWorkspace::sharedWorkspace().openURL(&url) };
            if opened {
                Ok(())
            } else {
                Err(OpenLinkError::NoHandler)
            }
        })
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::OpenLinkError;

    pub(super) fn open(_url: &str) -> Result<(), OpenLinkError> {
        Err(OpenLinkError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenLinkError, check_link};

    #[test]
    fn http_and_https_are_accepted() {
        for url in [
            "http://example.com",
            "https://example.com/",
            "https://example.com/a?k=v#f",
            "http://host:8088/#/src--handlr/docs/design/plan.md",
            "http://user:pass@host/",
            "http://[::1]:8080/",
            "https://h?q",
            "https://h#f",
        ] {
            assert_eq!(check_link(url), Ok(()), "{url}");
        }
    }

    #[test]
    fn upper_case_schemes_are_accepted() {
        for url in [
            "HTTP://example.com/",
            "HTTPS://example.com/",
            "HtTpS://example.com/",
        ] {
            assert_eq!(check_link(url), Ok(()), "{url}");
        }
    }

    #[test]
    fn a_missing_host_is_rejected() {
        for url in [
            "http://",
            "https:///path",
            "http://?q",
            "http://#f",
            "http://:8080/",
            "http://user@/",
            "http://[]/",
            "http://[::1/",
            "http:example.com",
            "https:/example.com",
            "example.com",
        ] {
            assert_eq!(check_link(url), Err(OpenLinkError::BadUrl), "{url}");
        }
    }

    #[test]
    fn whitespace_is_rejected() {
        for url in [
            "https://example.com/a b",
            " https://example.com/",
            "https://example.com/\t",
            "https://example.com/\n",
            "https://example.com/\u{a0}",
        ] {
            assert_eq!(check_link(url), Err(OpenLinkError::BadUrl), "{url:?}");
        }
    }

    #[test]
    fn control_characters_are_rejected() {
        for url in [
            "https://example.com/\0",
            "https://example.com/\u{7}",
            "https://example.com/\u{1b}",
            "https://example.com/\u{7f}",
            "https://exa\u{85}mple.com/",
        ] {
            assert_eq!(check_link(url), Err(OpenLinkError::BadUrl), "{url:?}");
        }
    }

    #[test]
    fn other_schemes_are_rejected() {
        for url in [
            "file:///etc/passwd",
            "file://host/etc/passwd",
            "content://com.example.provider/x",
            "intent://scan/#Intent;scheme=zxing;end",
            "javascript:alert(1)",
            "javascript://example.com/%0aalert(1)",
            "mailto:someone@example.com",
            "tel:+15555550100",
            "ftp://example.com/",
            "httpx://example.com/",
            "notascheme://x",
            "://example.com",
        ] {
            assert_eq!(check_link(url), Err(OpenLinkError::BadUrl), "{url}");
        }
    }

    #[test]
    fn an_empty_string_is_rejected() {
        assert_eq!(check_link(""), Err(OpenLinkError::BadUrl));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn other_desktops_are_unsupported() {
        assert_eq!(
            super::open_link("https://example.com/"),
            Err(OpenLinkError::Unsupported)
        );
        assert_eq!(super::open_link("file:///x"), Err(OpenLinkError::BadUrl));
    }
}
