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
