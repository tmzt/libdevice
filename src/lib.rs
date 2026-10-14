//! Deterministic device setup for the desktop-backed mobile simulator.
//!
//! By default the crate is data only: the simulated screen's geometry and
//! device specifics, with no GPU dependency. The `wgpu` feature adds the
//! offscreen `Device`, which keeps the device contract independent of a
//! windowing toolkit while matching the target portrait geometry used by the
//! eventual touch overlay.

#[cfg(feature = "wgpu")]
mod gpu;

#[cfg(feature = "wgpu")]
pub use gpu::{Device, DeviceConfig, DeviceError};

include!(concat!(env!("OUT_DIR"), "/devices.rs"));

/// Portrait dimensions used by the Samsung Galaxy S24 simulator target.
pub const GALAXY_S24_PORTRAIT: PortraitGeometry = GALAXY_S24.geometry;

/// Specifics of a simulated device, generated from `devices.toml`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceSpec {
    /// The device's key in `devices.toml`.
    pub id: &'static str,
    /// Human-readable device name.
    pub name: &'static str,
    pub geometry: PortraitGeometry,
}

impl DeviceSpec {
    /// Look up a device by its `devices.toml` key.
    #[must_use]
    pub fn by_id(id: &str) -> Option<Self> {
        DEVICES.iter().copied().find(|device| device.id == id)
    }
}

/// Pixel dimensions for the simulated device surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PortraitGeometry {
    pub width: u32,
    pub height: u32,
}

#[cfg(feature = "wgpu")]
impl PortraitGeometry {
    #[must_use]
    pub const fn extent(self) -> wgpu::Extent3d {
        wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DEVICE, DEVICES, DeviceSpec, GALAXY_S24};

    #[test]
    fn s24_matches_devices_toml() {
        assert_eq!(GALAXY_S24.id, "galaxy_s24");
        assert_eq!(GALAXY_S24.geometry.width, 1080);
        assert_eq!(GALAXY_S24.geometry.height, 2340);
        assert_eq!(DEFAULT_DEVICE, GALAXY_S24);
    }

    #[test]
    fn devices_are_found_by_id() {
        for device in DEVICES {
            assert_eq!(DeviceSpec::by_id(device.id), Some(*device));
        }
        assert_eq!(DeviceSpec::by_id("missing"), None);
    }
}

#[cfg(all(test, feature = "wgpu"))]
mod extent_tests {
    use super::{GALAXY_S24_PORTRAIT, PortraitGeometry};

    #[test]
    fn s24_extent_is_single_layer() {
        assert_eq!(GALAXY_S24_PORTRAIT.extent().depth_or_array_layers, 1);
    }

    #[test]
    fn custom_geometry_produces_matching_extent() {
        let geometry = PortraitGeometry {
            width: 320,
            height: 640,
        };
        assert_eq!(geometry.extent().width, 320);
        assert_eq!(geometry.extent().height, 640);
    }
}
