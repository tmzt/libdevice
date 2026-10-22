# Changes

Planned work, feature plans and architectural changes, appended in order.
Entries are never edited.

## 2026-10-15 23:29 EDT — Device pixel density

Plan: `docs/design/20261015_01_DEVICE_PIXEL_DENSITY_PLAN.md` (Draft).

- New plan: a required `ppi` for every device in `devices.toml`, carried
  on `DeviceSpec`; `px_per_mm`, `mm_to_px`, `px_to_mm` and
  `physical_size_mm` in device pixels; and a `--device-ppi` override.
- Part of the owner's Mac trackpad touch emulation request. Agreed with
  libmorph: libdevice holds only facts about the emulated phone, and libmorph
  owns all host input, including the trackpad mapping
  (`--morph-trackpad <physical|fit|off>`, libmorph plan
  `20261015_01_MAC_TRACKPAD_TOUCH_PLAN.md`).
- Also answers Noble's Keep's initial UI plan, open question 3 (pixel density
  for the 3 mm offset tool).

## 2026-10-22 19:12 EDT — Secure store and key store on the desktop (proposed)

Plan: `docs/design/20261022_01_SECURE_STORE_AND_KEY_STORE_PLAN.md` (Draft),
step 2 of libmorph's `20261022_04_OPAQUE_KEYS_PLAN.md`.

- The owner extended libmorph's layering rule to libdevice: desktop
  platform services are built here first, and libmorph only wraps them.
- This widens libdevice beyond facts about the emulated device, which the
  pixel density plan had set as the boundary. The new code is behind a
  `secure-store` feature, so the default build is unchanged.
- The macOS Keychain and memory backends move here from libmorph's
  unpushed `secure-store` branch. A key store with libmobile's API is
  added.
