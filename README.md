# libdevice

Deterministic offscreen GPU device setup for simulating a mobile display on [`wgpu`](https://github.com/gfx-rs/wgpu).

`libdevice` acquires a `wgpu` adapter, device and queue with a fixed, predictable set of limits, and pairs them with the pixel geometry of a simulated portrait screen. It renders offscreen by default, so the device contract stays independent of any windowing toolkit, but it can also select an adapter compatible with a window surface when you want to present.

## Features

- **Portrait geometry** — `PortraitGeometry` describes the simulated screen; `GALAXY_S24_PORTRAIT` (1080×2340) is the default.
- **Predictable limits** — downlevel defaults, with `max_texture_dimension_2d` raised to whatever the adapter supports so large frames can be rendered.
- **Offscreen targets** — `Device::create_color_target` allocates an `Rgba8Unorm` texture matching the geometry, usable as a render attachment and copy source.
- **Optional surfaces** — `new_with_surface` / `new_blocking_with_surface` pick an adapter that can present to a given `wgpu::Surface`.
- **Blocking constructors** — `new_blocking*` variants for tests and native tools, with no async runtime dependency.
- **Resizable** — `set_geometry` keeps the simulated geometry in sync with a resized window.

## Usage

```toml
[dependencies]
libdevice = { git = "https://github.com/tmzt/libdevice" }
```

```rust
use libdevice::{Device, DeviceConfig};

fn main() -> Result<(), libdevice::DeviceError> {
    let device = Device::new_blocking(DeviceConfig::default())?;
    println!("adapter: {:?}", device.adapter_info().name);

    let target = device.create_color_target("frame");
    // Render into `target` with device.device() / device.queue(),
    // then copy it out for comparison or screenshots.
    Ok(())
}
```

To present to a window, create the instance first so the surface and device share it:

```rust
let instance = libdevice::Device::create_instance();
let surface = instance.create_surface(window)?;
let device = libdevice::Device::new_blocking_with_surface(instance, config, &surface)?;
```

Set `DeviceConfig::force_fallback_adapter` to request a software adapter (useful on CI machines without a GPU).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
