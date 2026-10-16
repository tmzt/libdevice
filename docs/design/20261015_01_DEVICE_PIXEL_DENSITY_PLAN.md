---
title: Device pixel density — ppi per device and millimetre conversions
date: 2026-10-15
status: Implemented
covers:
  - A required `ppi` (physical pixels per inch) for every device in
    `devices.toml`, carried on `DeviceSpec`
  - Millimetre and pixel conversions on `DeviceSpec`: `px_per_mm`,
    `mm_to_px`, `px_to_mm`, `physical_size_mm`
  - A `--device-ppi` override in the existing `--device` options
  - The boundary with libmorph for Mac trackpad touch emulation: libdevice
    describes the emulated phone only, libmorph owns all host input
sources:
  - Owner's request (2026-10-15), relayed by the Noble's Keep session: "the
    touchpad/mobile emulation should be added to libdevice and libmorph ...
    following the standard rules for that repo (or the plan rules if
    missing). it should emulate the same behavior as mobile using the desktop
    multi-finger support on Mac"
  - Agreement with the libmorph session (2026-10-15): libdevice holds only
    device facts (`ppi`, `--device-ppi` and the four conversions, all in
    device pixels); no touch-emulation settings in libdevice; the trackpad
    mapping is libmorph's `--morph-trackpad <physical|fit|off>`
  - ~/src/NoblesKeep/docs/design/20261015_01_INITIAL_UI_PLAN.md, section 7
    (the offset tool: a 9 mm touch circle with the working point 3 mm above
    it, `px = mm × ppi / 25.4`) and open question 3 (pixel density)
  - Manufacturer specifications: Samsung Galaxy S24, 6.2 in, 1080 × 2340,
    416 ppi; Apple iPhone 15, 6.1 in, 1179 × 2556, 460 ppi
  - The plan rules in ~/src/NoblesKeep/AGENTS.md (critical rules 0 to 4),
    used here because libdevice has no AGENTS.md of its own
related:
  - ~/src/libmorph/docs/design/20261015_01_MAC_TRACKPAD_TOUCH_PLAN.md
    (Draft): turns macOS trackpad fingers into libmorph `Touch` events.
    Depends on this plan for `px_per_mm` in its `physical` mapping, and falls
    back to `fit` until this plan is implemented
  - ~/src/NoblesKeep/docs/design/20261015_01_INITIAL_UI_PLAN.md (Draft):
    depends on this plan for converting the offset tool's millimetres to
    pixels on the emulated device
---

# Device pixel density

## 1. Overview

libdevice describes simulated phones: geometry, pixel ratio, insets, cutouts
and corners, compiled from `devices.toml`. It has no notion of physical size.
Two consumers need one:

- **Noble's Keep's offset tool** draws a 9 mm touch circle and puts the
  working point 3 mm above it. Millimetres need the display's pixel density.
- **libmorph's Mac trackpad emulation** can map the trackpad onto the
  emulated screen at 1:1 physical scale, so a finger moving 10 mm on the
  trackpad moves 10 mm on the emulated phone. That needs the phone's pixels
  per millimetre, alongside the trackpad's own size, which libmorph reads
  from macOS.

This plan adds the pixel density of each device and conversions between
millimetres and device pixels. Nothing else changes.

## 2. Boundary with libmorph

Agreed with the libmorph session on 2026-10-15:

| Concern | Owner |
|---|---|
| The emulated phone's pixel density and physical size | libdevice (this plan) |
| Reading macOS trackpad fingers, finger ids and phases | libmorph |
| Mapping trackpad space onto the emulated screen (`physical`, `fit`, `off`) and its option `--morph-trackpad` | libmorph |
| Scaling device pixels to the window surface when the window manager grants less than the device size | libmorph, the same way it scales the safe area |
| The real pixel density of an Android device at run time | libmorph and libmobile, if a later plan wants it; not libdevice |

libdevice gets no trackpad, cursor or touch-emulation types or options. The
mapping describes the host computer, not the phone.

## 3. Data

### 3.1 `devices.toml`

Every device table gains a required key:

```toml
ppi = 416 # physical pixels per inch of the display
```

- An integer or a float, positive and finite. A missing, zero, negative or
  non-numeric `ppi` fails the build with a message naming the key, like the
  existing checks in `build.rs`.
- The value is the display's physical density as the manufacturer states
  it. It is independent of `pixel_ratio`, which is the logical scale (dp,
  points, CSS pixels) and not a physical measure.
- Initial values: `galaxy_s24` 416, `iphone_15` 460.

`ppi` is added to `build.rs`'s list of known device fields and to the format
comment at the top of `devices.toml`.

### 3.2 `DeviceSpec`

A new public field, after `pixel_ratio`:

```rust
/// Physical pixels per inch of the display.
pub ppi: f32,
```

`DeviceSpec` stays `Clone, Copy, Debug, PartialEq`. The generated consts
(`GALAXY_S24`, `IPHONE_15`) carry the value from `devices.toml`.

### 3.3 Conversions

All in device pixels of the portrait display, the space every other
`DeviceSpec` length uses. None of them know about the window surface.

```rust
impl DeviceSpec {
    /// Physical pixels per millimetre: `ppi / 25.4`.
    pub const fn px_per_mm(&self) -> f32;
    /// Millimetres on the display to device pixels.
    pub const fn mm_to_px(&self, mm: f32) -> f32;
    /// Device pixels to millimetres on the display.
    pub const fn px_to_mm(&self, px: f32) -> f32;
    /// The display's width and height in millimetres, portrait.
    pub const fn physical_size_mm(&self) -> (f32, f32);
}
```

Expected figures:

| Device | `px_per_mm` | `physical_size_mm` | `mm_to_px(9.0)` | `mm_to_px(7.5)` |
|---|---|---|---|---|
| `GALAXY_S24` | 16.378 | 65.94 × 142.88 | 147.40 | 122.83 |
| `IPHONE_15` | 18.110 | 65.10 × 141.14 | 162.99 | 135.83 |

(7.5 mm is the offset tool's distance from the contact point to the working
point: the circle's 4.5 mm radius plus 3 mm.)

### 3.4 `--device-ppi`

A new override in `src/args.rs`, under whatever prefix the caller uses:

```
  --device-ppi <ppi>               Physical pixels per inch of the display
```

- The value must parse as a positive, finite `f32`; otherwise
  `DeviceArgsError::InvalidValue` with `expected: "a positive number"`.
- It applies on top of the selected device, in any order, like every other
  override.
- Changing `--device-width` or `--device-height` does not change `ppi`, so the
  physical size grows or shrinks with the pixel size. This is the intended
  reading: the same panel, cut to a different size.

## 4. Compatibility

- Adding a public field to `DeviceSpec` breaks code that builds a
  `DeviceSpec` with a struct literal. libmorph and Noble's Keep only use the
  generated consts and `parse_args`/`extract_args`, which keep working.
- A `devices.toml` entry without `ppi` no longer builds. The only entries are
  in this repository and both gain one in step 1.

## 5. Steps

1. **`ppi` in the data.** Add `ppi` to both devices in `devices.toml`, its
   validation and code generation in `build.rs`, and the `ppi` field on
   `DeviceSpec`. Test that `GALAXY_S24.ppi == 416.0` and
   `IPHONE_15.ppi == 460.0`, and check by hand that a missing or zero `ppi`
   fails the build with a message naming the key.
   **Done when:** `cargo test` passes and both checks fail the build with
   messages naming `galaxy_s24.ppi`.
2. **Conversions.** Add `px_per_mm`, `mm_to_px`, `px_to_mm` and
   `physical_size_mm` as `const fn`s. Test them against the table in 3.3 to
   within 0.01, and test that `px_to_mm(mm_to_px(x))` returns `x` to within
   1e-4 for both devices.
   **Done when:** those tests pass.
3. **`--device-ppi`.** Add the override and its help line. Test it with and
   without the prefix, under another prefix (`--sim-ppi`), and with `0`,
   `-1`, `inf` and `abc`, which are rejected.
   **Done when:** those tests pass and `DEVICE_ARGS_HELP` lists the option.
4. **Docs.** Document `ppi` and the conversions in the README's features
   list and the crate docs.
   **Done when:** `cargo doc --no-deps` builds with no warnings and the
   README describes `ppi`, the conversions and `--device-ppi`.

Each step is its own commit and passes `cargo fmt -- --check`,
`cargo clippy --all-targets` with no warnings and `cargo test` first.

## 6. Open questions

1. **Separate horizontal and vertical density:** Android reports `xdpi` and
   `ydpi` separately. Current phone panels have square pixels, so this plan
   uses one `ppi`. Should it be two fields now, so a non-square panel never
   needs a breaking change?

## Amendment — 2026-10-15 23:37 EDT: implementation started

The owner approved this plan for implementation on 2026-10-15, as written:
one `ppi` per device (open question 1 answered: no separate horizontal and
vertical density). Status is now `In implementation`; from this commit the
document is immutable except by amendment on the owner's instruction.

## Amendment — 2026-10-15 23:38 EDT: implemented

Every step is done and its **Done when:** holds. Status is now
`Implemented`. Commits:

1. `ppi` in the data: 94745f9
2. Conversions: 34084dd
3. `--device-ppi`: d7163ad
4. Docs: 73b1952
