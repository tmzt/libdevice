//! **Disk space** (feature `disk-space`): how much room the volume holding
//! a path has, in bytes.
//!
//! * macOS, Linux and other Unix: `statvfs(2)`, from any thread, in
//!   microseconds.
//! * Windows and wasm32: [`DiskSpaceError::Unsupported`], until a Windows
//!   app asks for it.
//!
//! The numbers only: grading them into disk pressure, and how often to
//! check, are libmorph's.
//!
//! On macOS, APFS keeps *purgeable* space (caches the system can drop)
//! that `statvfs` does not count as free, while Finder counts it as
//! available. [`DiskSpace::free`] can therefore be lower than Finder
//! shows; for deciding whether to trim a cache, that errs on the safe side.

use std::fmt;
use std::path::Path;

/// A volume's space, in bytes, from `statvfs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiskSpace {
    /// Bytes an unprivileged process can still write: `f_bavail * f_frsize`.
    pub free: u64,
    /// The volume's size: `f_blocks * f_frsize`.
    pub total: u64,
}

/// Why the space could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiskSpaceError {
    /// The path cannot be used: it does not exist, is not reachable
    /// (permissions, a non-directory component, a symlink loop), is too
    /// long, or holds a NUL byte. The message names the path.
    Path(String),
    /// No disk-space call on this desktop (Windows and wasm32, for now).
    Unsupported,
    /// The platform call failed for another reason; the message has the
    /// OS error.
    Platform(String),
}

impl fmt::Display for DiskSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(message) => write!(f, "disk space: bad path: {message}"),
            Self::Unsupported => {
                f.write_str("reading disk space is not supported on this platform")
            }
            Self::Platform(message) => write!(f, "reading disk space failed: {message}"),
        }
    }
}

impl std::error::Error for DiskSpaceError {}

/// The space of the volume holding `path` on this desktop:
/// `free = f_bavail * f_frsize`, `total = f_blocks * f_frsize`, each
/// saturating at `u64::MAX`.
///
/// macOS, Linux and other Unix, through `statvfs`; `Err(Unsupported)` on
/// Windows and wasm32. Any thread, microseconds. On macOS, `free` leaves
/// out APFS purgeable space (see the [module docs](self)).
pub fn space(path: &Path) -> Result<DiskSpace, DiskSpaceError> {
    platform::space(path)
}

/// `blocks * block_size`, saturating rather than wrapping.
#[cfg_attr(not(unix), allow(dead_code))]
fn bytes(blocks: u64, block_size: u64) -> u64 {
    blocks.saturating_mul(block_size)
}

#[cfg(unix)]
mod platform {
    use super::{DiskSpace, DiskSpaceError, bytes};
    use std::ffi::CString;
    use std::io;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// Widen a `statvfs` field, whose C type is 32 or 64 bits by platform.
    fn wide<T: Into<u64>>(value: T) -> u64 {
        value.into()
    }

    pub(super) fn space(path: &Path) -> Result<DiskSpace, DiskSpaceError> {
        let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
            DiskSpaceError::Path(format!("{}: contains a NUL byte", path.display()))
        })?;
        let mut stat = MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `c_path` is a NUL-terminated string that outlives the
        // call, and `stat` points to writable memory of the struct's size.
        let result = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
        if result != 0 {
            let error = io::Error::last_os_error();
            let message = format!("{}: {error}", path.display());
            return Err(match error.raw_os_error() {
                Some(
                    libc::ENOENT | libc::ENOTDIR | libc::EACCES | libc::ELOOP | libc::ENAMETOOLONG,
                ) => DiskSpaceError::Path(message),
                _ => DiskSpaceError::Platform(message),
            });
        }
        // SAFETY: statvfs returned 0, so it filled in the struct.
        let stat = unsafe { stat.assume_init() };
        let block_size = wide(stat.f_frsize);
        Ok(DiskSpace {
            free: bytes(wide(stat.f_bavail), block_size),
            total: bytes(wide(stat.f_blocks), block_size),
        })
    }
}

#[cfg(not(unix))]
mod platform {
    use super::{DiskSpace, DiskSpaceError};
    use std::path::Path;

    pub(super) fn space(_path: &Path) -> Result<DiskSpace, DiskSpaceError> {
        Err(DiskSpaceError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::{DiskSpaceError, bytes, space};

    #[cfg(unix)]
    #[test]
    fn temp_dir_has_space() {
        let space = space(&std::env::temp_dir()).expect("statvfs on the temp dir");
        assert!(space.total > 0, "{space:?}");
        assert!(space.free <= space.total, "{space:?}");
    }

    #[cfg(unix)]
    #[test]
    fn missing_path_is_a_path_error() {
        let missing = std::env::temp_dir().join("libdevice-disk-space-no-such-dir/x");
        assert!(
            matches!(space(&missing), Err(DiskSpaceError::Path(_))),
            "{:?}",
            space(&missing)
        );
    }

    #[cfg(unix)]
    #[test]
    fn nul_byte_is_a_path_error() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let path = std::path::Path::new(OsStr::from_bytes(b"/tmp/a\0b"));
        assert!(matches!(space(path), Err(DiskSpaceError::Path(_))));
    }

    #[cfg(not(unix))]
    #[test]
    fn other_desktops_are_unsupported() {
        assert_eq!(
            space(&std::env::temp_dir()),
            Err(DiskSpaceError::Unsupported)
        );
    }

    #[test]
    fn multiply_saturates() {
        assert_eq!(bytes(u64::MAX, 4096), u64::MAX);
        assert_eq!(bytes(4096, u64::MAX), u64::MAX);
        assert_eq!(bytes(u64::MAX, 1), u64::MAX);
        assert_eq!(bytes(1 << 20, 4096), 1 << 32);
        assert_eq!(bytes(0, u64::MAX), 0);
    }

    #[test]
    fn errors_display() {
        assert_eq!(
            DiskSpaceError::Path("/x: No such file or directory (os error 2)".into()).to_string(),
            "disk space: bad path: /x: No such file or directory (os error 2)"
        );
        assert_eq!(
            DiskSpaceError::Unsupported.to_string(),
            "reading disk space is not supported on this platform"
        );
        assert_eq!(
            DiskSpaceError::Platform("EIO".into()).to_string(),
            "reading disk space failed: EIO"
        );
    }
}
