//! Deterministic device setup for the desktop-backed mobile simulator.
//!
//! By default the crate is data only: the simulated screen's geometry and
//! device specifics, with no GPU dependency. The `wgpu` feature adds the
//! offscreen `Device`, which keeps the device contract independent of a
//! windowing toolkit while matching the target portrait geometry used by the
//! eventual touch overlay.

#[cfg(feature = "wgpu")]
mod gpu;
mod insets;

#[cfg(feature = "wgpu")]
pub use gpu::{Device, DeviceConfig, DeviceError};
pub use insets::{
    CornerShape, Corners, Cutout, CutoutShape, Edge, InsetType, Insets, LogicalInsets, Platform,
    Rect, SystemInsets,
};

include!(concat!(env!("OUT_DIR"), "/devices.rs"));

/// Portrait dimensions used by the Samsung Galaxy S24 simulator target.
pub const GALAXY_S24_PORTRAIT: PortraitGeometry = GALAXY_S24.geometry;

/// Specifics of a simulated device, generated from `devices.toml`.
///
/// Every length is in physical pixels of the portrait display.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeviceSpec {
    /// The device's key in `devices.toml`.
    pub id: &'static str,
    /// Human-readable device name.
    pub name: &'static str,
    pub platform: Platform,
    pub geometry: PortraitGeometry,
    /// Physical pixels per dp, iOS point or CSS pixel.
    pub pixel_ratio: f32,
    pub insets: SystemInsets,
    pub cutouts: &'static [Cutout],
    pub corners: Corners,
}

impl DeviceSpec {
    /// Look up a device by its `devices.toml` key.
    #[must_use]
    pub fn by_id(id: &str) -> Option<Self> {
        DEVICES.iter().copied().find(|device| device.id == id)
    }

    /// The insets of one type, combining stored types where it is a union.
    #[must_use]
    pub const fn insets(&self, inset_type: InsetType) -> Insets {
        self.insets.get(inset_type)
    }

    /// `env(safe-area-inset-*)` in CSS pixels, for a full-screen page with
    /// `viewport-fit=cover`.
    #[must_use]
    pub fn css_safe_area(&self) -> LogicalInsets {
        self.insets(InsetType::CssSafeArea)
            .to_logical(self.pixel_ratio)
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
    use super::{
        CornerShape, CutoutShape, DEFAULT_DEVICE, DEVICES, DeviceSpec, GALAXY_S24, IPHONE_15,
        InsetType, Insets, Platform,
    };

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

    #[test]
    fn display_cutout_is_derived_from_cutouts() {
        let [hole] = GALAXY_S24.cutouts else {
            panic!("expected one cutout");
        };
        assert_eq!(hole.shape, CutoutShape::Circle);
        assert_eq!(
            GALAXY_S24.insets(InsetType::DisplayCutout),
            Insets {
                top: hole.bounds.y + hole.bounds.height,
                ..Insets::ZERO
            }
        );
    }

    #[test]
    fn explicit_display_cutout_is_kept() {
        assert_eq!(IPHONE_15.insets(InsetType::DisplayCutout).top, 177);
    }

    #[test]
    fn combined_types_union_their_parts() {
        let insets = GALAXY_S24.insets;
        assert_eq!(
            GALAXY_S24.insets(InsetType::SystemBars),
            insets.status_bars.union(insets.navigation_bars)
        );
        assert_eq!(
            GALAXY_S24.insets(InsetType::SafeContent),
            GALAXY_S24
                .insets(InsetType::SafeDrawing)
                .union(GALAXY_S24.insets(InsetType::SafeGestures))
        );
    }

    #[test]
    fn iphone_safe_area_matches_uikit() {
        let safe_area = IPHONE_15
            .insets(InsetType::IosSafeArea)
            .to_logical(IPHONE_15.pixel_ratio);
        assert_eq!((safe_area.top, safe_area.bottom), (59.0, 34.0));
        assert_eq!(IPHONE_15.css_safe_area(), safe_area);
    }

    #[test]
    fn corner_shape_defaults_to_platform_curve() {
        assert_eq!(GALAXY_S24.platform, Platform::Android);
        assert_eq!(GALAXY_S24.corners.shape, CornerShape::Circular);
        assert_eq!(IPHONE_15.platform, Platform::Ios);
        assert_eq!(IPHONE_15.corners.shape, CornerShape::Continuous);
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
