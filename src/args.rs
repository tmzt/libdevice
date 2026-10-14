//! Command-line selection of the simulated device.
//!
//! `--device <name>` (or `--device-device <name>`) picks a device from
//! `devices.toml` and every other `--device-*` option overrides one of its
//! fields. See [`DEVICE_ARGS_HELP`]. The `*_with_prefix` variants swap
//! `device` for a wrapper's own prefix, so `--sim-device`, `--sim-width`.

use std::fmt;

use crate::{
    CornerShape, Corners, DEFAULT_DEVICE, DEVICES, DeviceSpec, Insets, Platform, SystemInsets,
};

/// The option prefix used by [`DeviceSpec::parse_args`] and
/// [`DeviceSpec::extract_args`].
pub const DEFAULT_PREFIX: &str = "device";

/// Usage text for the device options, to append to a program's own help.
/// [`device_args_help`] gives the same text for another prefix.
pub const DEVICE_ARGS_HELP: &str = "\
Device options (lengths in physical pixels):
  --device <name>                  Simulated device (default: the first in devices.toml)
  --device-device <name>           The same, spelled like the other options
  --device-width <px>              Portrait width
  --device-height <px>             Portrait height
  --device-pixel-ratio <ratio>     Physical pixels per dp / point / CSS pixel
  --device-platform <platform>     android | ios
  --device-<inset> <px...>         Insets as CSS shorthand: 1-4 values, comma separated
  --device-<inset>-<side> <px>     One side: top | right | bottom | left
      <inset> is status-bars, navigation-bars, caption-bar, system-gestures,
      mandatory-system-gestures, tappable-element, display-cutout or waterfall
  --device-corner-radius <px...>   Corner radii as CSS border-radius shorthand
  --device-corner-shape <shape>    circular | continuous
";

/// A malformed or unknown device option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeviceArgsError {
    UnknownDevice(String),
    UnknownOption(String),
    MissingValue(String),
    InvalidValue {
        option: String,
        value: String,
        expected: &'static str,
    },
    DuplicateDevice,
}

impl fmt::Display for DeviceArgsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownDevice(name) => {
                write!(formatter, "unknown device `{name}`; expected one of ")?;
                for (index, device) in DEVICES.iter().enumerate() {
                    let separator = if index == 0 { "" } else { ", " };
                    write!(formatter, "{separator}{}", device.id)?;
                }
                Ok(())
            }
            Self::UnknownOption(option) => write!(formatter, "unknown device option `{option}`"),
            Self::MissingValue(option) => write!(formatter, "`{option}` needs a value"),
            Self::InvalidValue {
                option,
                value,
                expected,
            } => write!(formatter, "`{option}` got `{value}`, expected {expected}"),
            Self::DuplicateDevice => formatter.write_str("device selected more than once"),
        }
    }
}

impl std::error::Error for DeviceArgsError {}

/// [`DEVICE_ARGS_HELP`] with `--device` replaced by `--<prefix>`.
#[must_use]
pub fn device_args_help(prefix: &str) -> String {
    DEVICE_ARGS_HELP.replace("--device", &format!("--{}", normalize(prefix)))
}

/// Accept a prefix written as `sim`, `--sim` or `--sim-`.
fn normalize(prefix: &str) -> &str {
    prefix.trim_matches('-')
}

impl DeviceSpec {
    /// Build a device from arguments that are all device options, written
    /// either with the `--device-` prefix (`--device-width 1080`) or without
    /// it (`--width=1080`). `--device <name>` or `--device-device <name>`
    /// selects the base device.
    pub fn parse_args<I>(args: I) -> Result<Self, DeviceArgsError>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self::parse_args_with_prefix(DEFAULT_PREFIX, args)
    }

    /// [`parse_args`](Self::parse_args) for options under another prefix:
    /// with `sim`, `--sim-width` or `--width`, and `--sim`, `--sim-device` or
    /// `--device` to select.
    pub fn parse_args_with_prefix<I>(prefix: &str, args: I) -> Result<Self, DeviceArgsError>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let prefix = normalize(prefix);
        let mut args = args.into_iter().map(Into::into);
        let mut options = Vec::new();
        while let Some(arg) = args.next() {
            let Some(body) = arg.strip_prefix("--") else {
                return Err(DeviceArgsError::UnknownOption(arg));
            };
            let (name, inline) = split_value(body);
            let key = option_key(prefix, name).unwrap_or(name).to_owned();
            let value = value(&arg, inline, &mut args)?;
            options.push((key, value));
        }
        resolve(prefix, options)
    }

    /// Take the `--device` and `--device-*` options out of a full argument
    /// list, such as `std::env::args()`, and return the device with every
    /// other argument in its original order. Arguments after `--` are left
    /// alone.
    pub fn extract_args<I>(args: I) -> Result<(Self, Vec<String>), DeviceArgsError>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        Self::extract_args_with_prefix(DEFAULT_PREFIX, args)
    }

    /// [`extract_args`](Self::extract_args) for options under another
    /// prefix: with `sim`, `--sim` and `--sim-*` are taken out.
    pub fn extract_args_with_prefix<I>(
        prefix: &str,
        args: I,
    ) -> Result<(Self, Vec<String>), DeviceArgsError>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let prefix = normalize(prefix);
        let mut args = args.into_iter().map(Into::into);
        let mut options = Vec::new();
        let mut rest = Vec::new();
        while let Some(arg) = args.next() {
            if arg == "--" {
                rest.push(arg);
                rest.extend(args.by_ref());
                break;
            }
            let Some((name, inline)) = arg.strip_prefix("--").map(split_value) else {
                rest.push(arg);
                continue;
            };
            let Some(key) = option_key(prefix, name) else {
                rest.push(arg);
                continue;
            };
            let key = key.to_owned();
            let value = value(&arg, inline, &mut args)?;
            options.push((key, value));
        }
        Ok((resolve(prefix, options)?, rest))
    }
}

/// The key of a prefixed option name: `<prefix>` alone is the `device`
/// selector, `<prefix>-<key>` is `<key>` (so `<prefix>-device` also selects).
fn option_key<'a>(prefix: &str, name: &'a str) -> Option<&'a str> {
    if name == prefix {
        return Some("device");
    }
    name.strip_prefix(prefix)?.strip_prefix('-')
}

/// Split `name=value` into the name and an inline value.
fn split_value(body: &str) -> (&str, Option<&str>) {
    match body.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (body, None),
    }
}

/// An option's inline value, or else the next argument.
fn value(
    arg: &str,
    inline: Option<&str>,
    args: &mut impl Iterator<Item = String>,
) -> Result<String, DeviceArgsError> {
    let option = || arg.split('=').next().unwrap_or(arg).to_owned();
    match inline {
        Some(value) => Ok(value.to_owned()),
        None => args
            .next()
            .ok_or_else(|| DeviceArgsError::MissingValue(option())),
    }
}

/// Apply options, keyed without their prefix (`device` selects), to the
/// selected device, whatever order they came in.
fn resolve(prefix: &str, options: Vec<(String, String)>) -> Result<DeviceSpec, DeviceArgsError> {
    let mut device = DEFAULT_DEVICE;
    let mut selected = false;
    for (key, value) in &options {
        if key == "device" {
            if selected {
                return Err(DeviceArgsError::DuplicateDevice);
            }
            selected = true;
            device = find(value).ok_or_else(|| DeviceArgsError::UnknownDevice(value.clone()))?;
        }
    }
    for (key, value) in &options {
        if key != "device" {
            apply(&mut device, &format!("--{prefix}-{key}"), key, value)?;
        }
    }
    Ok(device)
}

/// Match a device by id, ignoring case and `-`/`_`, so `galaxy-s24` and the
/// const name `GALAXY_S24` both work.
fn find(name: &str) -> Option<DeviceSpec> {
    let name = name.to_ascii_lowercase().replace('-', "_");
    DeviceSpec::by_id(&name)
}

/// Apply one override; `option` is its full name for error messages.
fn apply(
    device: &mut DeviceSpec,
    option: &str,
    key: &str,
    value: &str,
) -> Result<(), DeviceArgsError> {
    let invalid = |expected| DeviceArgsError::InvalidValue {
        option: option.to_owned(),
        value: value.to_owned(),
        expected,
    };
    let pixels = |value: &str| value.trim().parse::<u32>().ok();
    let positive = |value: &str| pixels(value).filter(|&value| value > 0);

    match key {
        "width" => {
            device.geometry.width = positive(value).ok_or_else(|| invalid("a positive integer"))?
        }
        "height" => {
            device.geometry.height = positive(value).ok_or_else(|| invalid("a positive integer"))?
        }
        "pixel-ratio" => {
            device.pixel_ratio = value
                .parse::<f32>()
                .ok()
                .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
                .ok_or_else(|| invalid("a positive number"))?;
        }
        "platform" => {
            device.platform = match value {
                "android" => Platform::Android,
                "ios" => Platform::Ios,
                _ => return Err(invalid("android or ios")),
            };
        }
        "corner-radius" => {
            let [top_left, top_right, bottom_right, bottom_left] =
                shorthand(value).ok_or_else(|| invalid("1-4 non-negative integers"))?;
            device.corners = Corners {
                top_left,
                top_right,
                bottom_right,
                bottom_left,
                ..device.corners
            };
        }
        "corner-shape" => {
            device.corners.shape = match value {
                "circular" => CornerShape::Circular,
                "continuous" => CornerShape::Continuous,
                _ => return Err(invalid("circular or continuous")),
            };
        }
        _ => {
            if let Some(insets) = inset_mut(&mut device.insets, key) {
                let [top, right, bottom, left] =
                    shorthand(value).ok_or_else(|| invalid("1-4 non-negative integers"))?;
                *insets = Insets {
                    top,
                    right,
                    bottom,
                    left,
                };
                return Ok(());
            }
            let side = key
                .rsplit_once('-')
                .and_then(|(inset, side)| Some((inset_mut(&mut device.insets, inset)?, side)));
            let Some((insets, side)) = side else {
                return Err(DeviceArgsError::UnknownOption(option.to_owned()));
            };
            let side = match side {
                "top" => &mut insets.top,
                "right" => &mut insets.right,
                "bottom" => &mut insets.bottom,
                "left" => &mut insets.left,
                _ => return Err(DeviceArgsError::UnknownOption(option.to_owned())),
            };
            *side = pixels(value).ok_or_else(|| invalid("a non-negative integer"))?;
        }
    }
    Ok(())
}

fn inset_mut<'a>(insets: &'a mut SystemInsets, name: &str) -> Option<&'a mut Insets> {
    Some(match name {
        "status-bars" => &mut insets.status_bars,
        "navigation-bars" => &mut insets.navigation_bars,
        "caption-bar" => &mut insets.caption_bar,
        "system-gestures" => &mut insets.system_gestures,
        "mandatory-system-gestures" => &mut insets.mandatory_system_gestures,
        "tappable-element" => &mut insets.tappable_element,
        "display-cutout" => &mut insets.display_cutout,
        "waterfall" => &mut insets.waterfall,
        _ => return None,
    })
}

/// Expand CSS box shorthand (`a`, `a,b`, `a,b,c` or `a,b,c,d`, commas or
/// spaces) clockwise from the top, or the top left for radii.
fn shorthand(value: &str) -> Option<[u32; 4]> {
    let values = value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().ok())
        .collect::<Option<Vec<u32>>>()?;
    Some(match values[..] {
        [all] => [all; 4],
        [vertical, horizontal] => [vertical, horizontal, vertical, horizontal],
        [top, horizontal, bottom] => [top, horizontal, bottom, horizontal],
        [top, right, bottom, left] => [top, right, bottom, left],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{DEVICE_ARGS_HELP, DeviceArgsError, device_args_help};
    use crate::{CornerShape, DEFAULT_DEVICE, DeviceSpec, GALAXY_S24, IPHONE_15, Insets};

    #[test]
    fn no_args_is_the_default_device() {
        assert_eq!(DeviceSpec::parse_args::<[&str; 0]>([]), Ok(DEFAULT_DEVICE));
    }

    #[test]
    fn device_selects_by_id_or_const_name() {
        for name in ["iphone_15", "iphone-15", "IPHONE_15"] {
            assert_eq!(DeviceSpec::parse_args(["--device", name]), Ok(IPHONE_15));
        }
        assert_eq!(
            DeviceSpec::parse_args(["--device=pixel"]),
            Err(DeviceArgsError::UnknownDevice("pixel".into()))
        );
    }

    #[test]
    fn prefixed_and_bare_options_agree() {
        let prefixed = DeviceSpec::parse_args([
            "--device-width=720",
            "--device-pixel-ratio",
            "2",
            "--device-status-bars-top=80",
        ]);
        let bare = DeviceSpec::parse_args([
            "--width",
            "720",
            "--pixel-ratio=2",
            "--status-bars-top",
            "80",
        ]);
        assert_eq!(prefixed, bare);
        let device = prefixed.unwrap();
        assert_eq!(device.geometry.width, 720);
        assert_eq!(device.geometry.height, GALAXY_S24.geometry.height);
        assert_eq!(device.pixel_ratio, 2.0);
        assert_eq!(device.insets.status_bars.top, 80);
    }

    #[test]
    fn overrides_apply_to_the_selected_device_in_any_order() {
        let device =
            DeviceSpec::parse_args(["--device-height=2000", "--device", "iphone_15"]).unwrap();
        assert_eq!(device.id, "iphone_15");
        assert_eq!(device.geometry.height, 2000);
        assert_eq!(
            DeviceSpec::parse_args(["--device=iphone_15", "--device=galaxy_s24"]),
            Err(DeviceArgsError::DuplicateDevice)
        );
    }

    #[test]
    fn insets_and_corners_take_css_shorthand() {
        let device = DeviceSpec::parse_args([
            "--waterfall=1,2,3",
            "--navigation-bars=0 0 48",
            "--corner-radius=10,20",
            "--corner-shape=continuous",
        ])
        .unwrap();
        assert_eq!(
            device.insets.waterfall,
            Insets {
                top: 1,
                right: 2,
                bottom: 3,
                left: 2
            }
        );
        assert_eq!(
            device.insets.navigation_bars,
            Insets {
                top: 0,
                right: 0,
                bottom: 48,
                left: 0
            }
        );
        let corners = device.corners;
        assert_eq!(
            [
                corners.top_left,
                corners.top_right,
                corners.bottom_right,
                corners.bottom_left
            ],
            [10, 20, 10, 20]
        );
        assert_eq!(corners.shape, CornerShape::Continuous);
    }

    #[test]
    fn bad_options_are_reported() {
        assert_eq!(
            DeviceSpec::parse_args(["--device-depth=3"]),
            Err(DeviceArgsError::UnknownOption("--device-depth".into()))
        );
        assert_eq!(
            DeviceSpec::parse_args(["--status-bars-middle=3"]),
            Err(DeviceArgsError::UnknownOption(
                "--device-status-bars-middle".into()
            ))
        );
        assert_eq!(
            DeviceSpec::parse_args(["--device-width"]),
            Err(DeviceArgsError::MissingValue("--device-width".into()))
        );
        assert!(matches!(
            DeviceSpec::parse_args(["--width=0"]),
            Err(DeviceArgsError::InvalidValue { .. })
        ));
        assert!(matches!(
            DeviceSpec::parse_args(["--waterfall=1,2,3,4,5"]),
            Err(DeviceArgsError::InvalidValue { .. })
        ));
        assert_eq!(
            DeviceSpec::parse_args(["width=3"]),
            Err(DeviceArgsError::UnknownOption("width=3".into()))
        );
    }

    #[test]
    fn extract_leaves_other_args_in_place() {
        let (device, rest) = DeviceSpec::extract_args([
            "sim",
            "--device",
            "iphone-15",
            "--verbose",
            "--width=10",
            "--device-pixel-ratio=2",
            "scene.toml",
            "--",
            "--device-width=1",
        ])
        .unwrap();
        assert_eq!(device.id, "iphone_15");
        assert_eq!(device.geometry.width, IPHONE_15.geometry.width);
        assert_eq!(device.pixel_ratio, 2.0);
        assert_eq!(
            rest,
            [
                "sim",
                "--verbose",
                "--width=10",
                "scene.toml",
                "--",
                "--device-width=1"
            ]
        );
    }

    #[test]
    fn device_device_selects() {
        assert_eq!(
            DeviceSpec::parse_args(["--device-device", "iphone_15"]),
            Ok(IPHONE_15)
        );
        let (device, rest) =
            DeviceSpec::extract_args(["sim", "--device-device=iphone_15"]).unwrap();
        assert_eq!((device, rest), (IPHONE_15, vec!["sim".to_owned()]));
        assert_eq!(
            DeviceSpec::parse_args(["--device=iphone_15", "--device-device=galaxy_s24"]),
            Err(DeviceArgsError::DuplicateDevice)
        );
    }

    #[test]
    fn other_prefixes_select_and_override() {
        for prefix in ["sim", "--sim", "--sim-"] {
            for selector in ["--sim", "--sim-device"] {
                let (device, rest) = DeviceSpec::extract_args_with_prefix(
                    prefix,
                    [
                        "app",
                        selector,
                        "iphone_15",
                        "--sim-width=10",
                        "--device-width=20",
                    ],
                )
                .unwrap();
                assert_eq!(device.id, "iphone_15");
                assert_eq!(device.geometry.width, 10);
                assert_eq!(rest, ["app", "--device-width=20"]);
            }
        }
        // Filtered input under a prefix may still drop it.
        let device = DeviceSpec::parse_args_with_prefix(
            "sim",
            ["--device=iphone_15", "--width=10", "--sim-height=20"],
        )
        .unwrap();
        assert_eq!(device.id, "iphone_15");
        assert_eq!((device.geometry.width, device.geometry.height), (10, 20));
        // A longer option sharing the prefix's letters is not ours.
        assert_eq!(
            DeviceSpec::extract_args_with_prefix("sim", ["--simulate"])
                .unwrap()
                .1,
            ["--simulate"]
        );
    }

    #[test]
    fn errors_name_the_prefix_in_use() {
        assert_eq!(
            DeviceSpec::extract_args_with_prefix("sim", ["--sim-depth=3"]),
            Err(DeviceArgsError::UnknownOption("--sim-depth".into()))
        );
        assert!(device_args_help("--sim").contains("  --sim-device <name>"));
        assert_eq!(device_args_help("device"), DEVICE_ARGS_HELP);
    }

    #[test]
    fn extract_rejects_unknown_device_options() {
        assert_eq!(
            DeviceSpec::extract_args(["sim", "--device-colour=red"]),
            Err(DeviceArgsError::UnknownOption("--device-colour".into()))
        );
    }
}
