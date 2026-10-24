# libdevice

Data describing simulated mobile devices — screen geometry, pixel ratio, insets, cutouts and display corners — compiled from [`devices.toml`](devices.toml) into const data, plus helpers for selecting and overriding a device from `--device` command-line options.

`libdevice` has no runtime dependencies and no GPU or windowing code; renderers own their own GPU handles and read the device data from here.

## Features

- **Device specifics** — each device in [`devices.toml`](devices.toml) is compiled into a `DeviceSpec` const (`GALAXY_S24`), listed in `DEVICES` and found by key with `DeviceSpec::by_id`. The first entry is `DEFAULT_DEVICE`.
- **Insets, cutouts and corners** — every Android `WindowInsets` type is stored per device, and `DeviceSpec::insets(InsetType::…)` also derives the combined types (`SystemBars`, `SafeDrawing`, `SafeGestures`, `SafeContent`), the iOS safe area and CSS `env(safe-area-inset-*)`; `css_safe_area()` gives the latter in CSS pixels. Cutouts are circles, capsules, notches, rectangles or SVG paths, and corners are circular (Android) or continuous (iOS).
- **Portrait geometry** — `PortraitGeometry` describes the simulated screen; `GALAXY_S24_PORTRAIT` is the S24's (1080×2340).
- **Physical size** — every device has its display's `ppi` (the manufacturer's figure), and `px_per_mm`, `mm_to_px`, `px_to_mm` and `physical_size_mm` convert between millimetres and device pixels, for sizes that should match a finger rather than a pixel count. `--device-ppi` overrides it.
- **Command-line selection** — `--device <name>` and `--device-*` overrides, parsed from filtered options or extracted from a full argument list, under any prefix.
- **App directories** — `dirs::data_dir(app_id)` and `dirs::cache_dir(app_id)` give the per-user directories for files an app keeps and files it can recreate: `~/Library/Application Support/<id>` and `~/Library/Caches/<id>` on macOS, `$XDG_DATA_HOME/<id>` and `$XDG_CACHE_HOME/<id>` (defaults `~/.local/share`, `~/.cache`) on Linux and other Unix, `%APPDATA%\<id>` and `%LOCALAPPDATA%\<id>\cache` on Windows, and `None` elsewhere or for an invalid id. Paths only; nothing is created.
- **Opening links** (feature `open-link`, off by default) — `link::open_link(url)` opens an http or https link in the default browser, with the same API as libmobile's on Android. It works on macOS; other desktops get `OpenLinkError::Unsupported`. `link::check_link` is the check it makes first.

## Usage

```toml
[dependencies]
libdevice = { git = "https://github.com/tmzt/libdevice" }
```

```rust
use libdevice::{GALAXY_S24, InsetType};

let screen = GALAXY_S24.geometry;
println!("{}x{}", screen.width, screen.height);

let safe = GALAXY_S24.insets(InsetType::SafeDrawing); // physical pixels
let css = GALAXY_S24.css_safe_area();                // CSS pixels

let touch_circle = GALAXY_S24.mm_to_px(9.0);         // 147.4 device pixels
```

### Command-line device selection

`--device <name>` (or `--device-device <name>`) picks a device by its `devices.toml` key (`galaxy_s24`, `galaxy-s24` and `GALAXY_S24` all work) and `--device-*` options override its fields; `DEVICE_ARGS_HELP` has the full list.

Wrappers with their own option prefix use `parse_args_with_prefix` / `extract_args_with_prefix`: with `"sim"`, `--sim <name>` or `--sim-device <name>` selects and `--sim-width` overrides, and `device_args_help("sim")` gives the matching help text.

```rust
// From the full argument list: device options are taken out, the rest returned in order.
let (device, rest) = libdevice::DeviceSpec::extract_args(std::env::args())?;

// From options already split out, with or without the prefix:
let device = libdevice::DeviceSpec::parse_args(["--device=iphone_15", "--status-bars-top=180"])?;
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Architectural decisions

| Document | Date | Status | Decision |
|---|---|---|---|
| [Device pixel density — ppi per device and millimetre conversions](docs/design/20261015_01_DEVICE_PIXEL_DENSITY_PLAN.md) | 2026-10-15 | Implemented | Adds a required `ppi` per device, mm↔px conversions and `--device-ppi`, leaving all trackpad input to libmorph; all four steps done. |
| [Secure store and key store on the desktop](docs/design/20261022_01_SECURE_STORE_AND_KEY_STORE_PLAN.md) | 2026-10-22 | Implemented | A `secure-store` feature gives the desktop the same blob store as libmobile on Android (macOS login Keychain, or memory), so libmorph only wraps it. Its key store half moved to libmorph's key store plan, whose engine libdevice copies. The first platform code in libdevice, on the owner's layering rule. Done and checked from a Terminal; released in 0.1.1. |
| [Opening a link in another app on the desktop](docs/design/20261023_01_OPEN_LINK_PLAN.md) | 2026-10-23 | Implemented | Feature `open-link`: `open_link(url)` with libmobile's API, through `NSWorkspace` on macOS and unsupported on other desktops, so libmorph can wrap both. Requested by handlr; released in libdevice 0.1.2. |
| [The app's data and cache directories on the desktop](docs/design/20261023_02_APP_DIRECTORIES_PLAN.md) | 2026-10-23 | Implemented | `dirs::data_dir(app_id)` and `cache_dir(app_id)`: each desktop's per-user, per-app paths (Application Support and Caches on macOS, XDG on Linux, APPDATA on Windows), computed only, for libmorph to create. Released in libdevice 0.1.2. |
