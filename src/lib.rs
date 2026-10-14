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

/// Portrait dimensions used by the Samsung Galaxy S24 simulator target.
pub const GALAXY_S24_PORTRAIT: PortraitGeometry = PortraitGeometry {
    width: 1080,
    height: 2340,
};

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

#[cfg(all(test, feature = "wgpu"))]
mod tests {
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
