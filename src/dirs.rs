//! **App directories**: the per-user, per-app places on the desktop for
//! files the app keeps ([`data_dir`]) and files it can recreate
//! ([`cache_dir`]), named by an app id such as `dev.tmzt.handlr`.
//!
//! | Platform | [`data_dir`] | [`cache_dir`] |
//! |---|---|---|
//! | macOS | `$HOME/Library/Application Support/<id>` | `$HOME/Library/Caches/<id>` |
//! | Linux and other Unix | `$XDG_DATA_HOME/<id>` (default `$HOME/.local/share/<id>`) | `$XDG_CACHE_HOME/<id>` (default `$HOME/.cache/<id>`) |
//! | Windows | `%APPDATA%\<id>` | `%LOCALAPPDATA%\<id>\cache` |
//! | Android, iOS, wasm and the rest | `None` | `None` |
//!
//! Paths only: nothing is created or checked on disk. The environment is
//! read once per call. Core, with no feature or dependency.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The per-user directory for files the app keeps:
/// macOS `~/Library/Application Support/<app_id>`;
/// Linux and other Unix `$XDG_DATA_HOME/<app_id>` (default
/// `~/.local/share/<app_id>`); Windows `%APPDATA%\<app_id>`.
///
/// `None` if the home directory (or `APPDATA`) is unknown, on targets with
/// no desktop rule (Android, iOS, wasm), or when `app_id` is not a valid
/// name: 1 to 64 of `[A-Za-z0-9._-]`, not `.` or `..`. Nothing is created.
#[must_use]
pub fn data_dir(app_id: &str) -> Option<PathBuf> {
    resolve(Rule::CURRENT, Kind::Data, app_id, |name| {
        std::env::var_os(name)
    })
}

/// The per-user directory for files the app can recreate:
/// macOS `~/Library/Caches/<app_id>`; Linux and other Unix
/// `$XDG_CACHE_HOME/<app_id>` (default `~/.cache/<app_id>`); Windows
/// `%LOCALAPPDATA%\<app_id>\cache`.
///
/// `None` in the same cases as [`data_dir`] (with `LOCALAPPDATA` on
/// Windows). Nothing is created.
#[must_use]
pub fn cache_dir(app_id: &str) -> Option<PathBuf> {
    resolve(Rule::CURRENT, Kind::Cache, app_id, |name| {
        std::env::var_os(name)
    })
}

/// Which platform's rule applies. Each target uses one; the tests use all.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    MacOs,
    Xdg,
    Windows,
    Unsupported,
}

impl Rule {
    #[cfg(target_os = "macos")]
    const CURRENT: Self = Self::MacOs;
    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    ))]
    const CURRENT: Self = Self::Xdg;
    #[cfg(windows)]
    const CURRENT: Self = Self::Windows;
    #[cfg(not(any(unix, windows)))]
    const CURRENT: Self = Self::Unsupported;
    #[cfg(any(target_os = "ios", target_os = "android"))]
    const CURRENT: Self = Self::Unsupported;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Data,
    Cache,
}

/// A valid app id is one path component: 1 to 64 of `[A-Za-z0-9._-]`,
/// not `.` or `..`.
fn valid_id(app_id: &str) -> bool {
    (1..=64).contains(&app_id.len())
        && app_id != "."
        && app_id != ".."
        && app_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// The rule itself, a pure function of the platform and an environment
/// lookup, so every platform's rule is tested on any host.
fn resolve(
    rule: Rule,
    kind: Kind,
    app_id: &str,
    env: impl Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    if !valid_id(app_id) {
        return None;
    }
    // An empty value counts as unset.
    let var = |name: &str| env(name).filter(|value| !value.is_empty());
    let home = || var("HOME").map(PathBuf::from);
    let base = match (rule, kind) {
        (Rule::MacOs, Kind::Data) => home()?.join("Library/Application Support"),
        (Rule::MacOs, Kind::Cache) => home()?.join("Library/Caches"),
        (Rule::Xdg, Kind::Data) => {
            xdg(&var, "XDG_DATA_HOME").or_else(|| Some(home()?.join(".local/share")))?
        }
        (Rule::Xdg, Kind::Cache) => {
            xdg(&var, "XDG_CACHE_HOME").or_else(|| Some(home()?.join(".cache")))?
        }
        (Rule::Windows, Kind::Data) => PathBuf::from(var("APPDATA")?),
        (Rule::Windows, Kind::Cache) => {
            return Some(
                PathBuf::from(var("LOCALAPPDATA")?)
                    .join(app_id)
                    .join("cache"),
            );
        }
        (Rule::Unsupported, _) => return None,
    };
    Some(base.join(app_id))
}

/// An `XDG_*` base directory, ignored unless absolute, as the XDG Base
/// Directory Specification says.
fn xdg(var: &impl Fn(&str) -> Option<OsString>, name: &str) -> Option<PathBuf> {
    var(name)
        .map(PathBuf::from)
        .filter(|path| path_is_absolute(path))
}

/// Absolute in the Unix sense (a leading `/`), so the XDG rule behaves
/// the same when it is tested on Windows.
fn path_is_absolute(path: &Path) -> bool {
    path.as_os_str().as_encoded_bytes().first() == Some(&b'/')
}

#[cfg(test)]
mod tests {
    use super::{Kind, Rule, resolve, valid_id};
    use std::ffi::OsString;
    use std::path::PathBuf;

    const ID: &str = "dev.tmzt.handlr";

    fn run(rule: Rule, kind: Kind, id: &str, vars: &[(&str, &str)]) -> Option<PathBuf> {
        resolve(rule, kind, id, |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        })
    }

    fn path(parts: &[&str]) -> Option<PathBuf> {
        Some(parts.iter().collect())
    }

    #[test]
    fn macos_uses_the_library() {
        let env = [("HOME", "/Users/me"), ("XDG_DATA_HOME", "/xdg")];
        assert_eq!(
            run(Rule::MacOs, Kind::Data, ID, &env),
            path(&["/Users/me", "Library", "Application Support", ID])
        );
        assert_eq!(
            run(Rule::MacOs, Kind::Cache, ID, &env),
            path(&["/Users/me", "Library", "Caches", ID])
        );
    }

    #[test]
    fn xdg_uses_its_variables() {
        let env = [
            ("HOME", "/home/me"),
            ("XDG_DATA_HOME", "/data"),
            ("XDG_CACHE_HOME", "/cache"),
        ];
        assert_eq!(run(Rule::Xdg, Kind::Data, ID, &env), path(&["/data", ID]));
        assert_eq!(run(Rule::Xdg, Kind::Cache, ID, &env), path(&["/cache", ID]));
    }

    #[test]
    fn xdg_defaults_when_unset_relative_or_empty() {
        for (data, cache) in [
            (None, None),
            (Some("rel/data"), Some("cache")),
            (Some(""), Some("")),
        ] {
            let mut env = vec![("HOME", "/home/me")];
            if let Some(data) = data {
                env.push(("XDG_DATA_HOME", data));
            }
            if let Some(cache) = cache {
                env.push(("XDG_CACHE_HOME", cache));
            }
            assert_eq!(
                run(Rule::Xdg, Kind::Data, ID, &env),
                path(&["/home/me", ".local", "share", ID])
            );
            assert_eq!(
                run(Rule::Xdg, Kind::Cache, ID, &env),
                path(&["/home/me", ".cache", ID])
            );
        }
    }

    #[test]
    fn xdg_without_home_needs_absolute_variables() {
        let env = [("XDG_DATA_HOME", "/data"), ("XDG_CACHE_HOME", "relative")];
        assert_eq!(run(Rule::Xdg, Kind::Data, ID, &env), path(&["/data", ID]));
        assert_eq!(run(Rule::Xdg, Kind::Cache, ID, &env), None);
    }

    #[test]
    fn missing_home_gives_none() {
        for env in [&[][..], &[("HOME", "")][..]] {
            for rule in [Rule::MacOs, Rule::Xdg] {
                for kind in [Kind::Data, Kind::Cache] {
                    assert_eq!(run(rule, kind, ID, env), None, "{rule:?} {kind:?}");
                }
            }
        }
    }

    #[test]
    fn windows_uses_appdata() {
        let env = [
            ("APPDATA", r"C:\Users\me\AppData\Roaming"),
            ("LOCALAPPDATA", r"C:\Users\me\AppData\Local"),
            ("HOME", "/home/me"),
        ];
        assert_eq!(
            run(Rule::Windows, Kind::Data, ID, &env),
            path(&[r"C:\Users\me\AppData\Roaming", ID])
        );
        assert_eq!(
            run(Rule::Windows, Kind::Cache, ID, &env),
            path(&[r"C:\Users\me\AppData\Local", ID, "cache"])
        );
    }

    #[test]
    fn missing_appdata_gives_none() {
        let env = [("HOME", "/home/me")];
        assert_eq!(run(Rule::Windows, Kind::Data, ID, &env), None);
        assert_eq!(run(Rule::Windows, Kind::Cache, ID, &env), None);
        let env = [("APPDATA", ""), ("LOCALAPPDATA", "")];
        assert_eq!(run(Rule::Windows, Kind::Data, ID, &env), None);
        assert_eq!(run(Rule::Windows, Kind::Cache, ID, &env), None);
    }

    #[test]
    fn unsupported_targets_give_none() {
        let env = [("HOME", "/home/me"), ("APPDATA", "/appdata")];
        assert_eq!(run(Rule::Unsupported, Kind::Data, ID, &env), None);
        assert_eq!(run(Rule::Unsupported, Kind::Cache, ID, &env), None);
    }

    #[test]
    fn bad_ids_give_none() {
        let long = "a".repeat(65);
        let env = [
            ("HOME", "/home/me"),
            ("APPDATA", "/appdata"),
            ("LOCALAPPDATA", "/local"),
        ];
        for id in [
            "",
            long.as_str(),
            "a/b",
            "../x",
            "..",
            ".",
            "a b",
            "caf\u{e9}",
            r"a\b",
        ] {
            assert!(!valid_id(id), "{id:?}");
            for rule in [Rule::MacOs, Rule::Xdg, Rule::Windows] {
                for kind in [Kind::Data, Kind::Cache] {
                    assert_eq!(run(rule, kind, id, &env), None, "{id:?}");
                }
            }
        }
    }

    #[test]
    fn good_ids_are_accepted() {
        for id in ["a", "...", ".hidden", "A-b_c.9", &"z".repeat(64)] {
            assert!(valid_id(id), "{id:?}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_current_target_uses_its_rule() {
        assert_eq!(Rule::CURRENT, Rule::MacOs);
    }
}
