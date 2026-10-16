//! Data describing simulated mobile devices.
//!
//! Each device in `devices.toml` is compiled into a const [`DeviceSpec`]: its
//! portrait geometry, pixel ratio, pixel density, insets, cutouts and display
//! corners. [`DeviceSpec::mm_to_px`] and its siblings convert physical
//! millimetres on the display to device pixels and back.
//! [`DeviceSpec::parse_args`] and [`DeviceSpec::extract_args`] select and
//! override one from `--device` command-line options.

mod args;
mod insets;

pub use args::{DEFAULT_PREFIX, DEVICE_ARGS_HELP, DeviceArgsError, device_args_help};

pub use insets::{
    CornerShape, Corners, Cutout, CutoutShape, Edge, InsetType, Insets, LogicalInsets, Platform,
    Rect, SystemInsets,
};

include!(concat!(env!("OUT_DIR"), "/devices.rs"));

/// Portrait dimensions used by the Samsung Galaxy S24 simulator target.
pub const GALAXY_S24_PORTRAIT: PortraitGeometry = GALAXY_S24.geometry;

const MM_PER_INCH: f32 = 25.4;

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
    /// Physical pixels per inch of the display.
    pub ppi: f32,
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

    /// Physical pixels per millimetre of the display: `ppi / 25.4`.
    #[must_use]
    pub const fn px_per_mm(&self) -> f32 {
        self.ppi / MM_PER_INCH
    }

    /// Millimetres on the display to device pixels.
    #[must_use]
    pub const fn mm_to_px(&self, mm: f32) -> f32 {
        mm * self.px_per_mm()
    }

    /// Device pixels to millimetres on the display.
    #[must_use]
    pub const fn px_to_mm(&self, px: f32) -> f32 {
        px / self.px_per_mm()
    }

    /// The display's width and height in millimetres, portrait.
    #[must_use]
    pub const fn physical_size_mm(&self) -> (f32, f32) {
        (
            self.px_to_mm(self.geometry.width as f32),
            self.px_to_mm(self.geometry.height as f32),
        )
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
    fn ppi_comes_from_devices_toml() {
        assert_eq!(GALAXY_S24.ppi, 416.0);
        assert_eq!(IPHONE_15.ppi, 460.0);
    }

    /// The figures in the density plan, section 3.3.
    #[test]
    fn conversions_match_the_density_plan() {
        let close = |actual: f32, expected: f32| {
            assert!(
                (actual - expected).abs() < 0.01,
                "{actual} is not {expected}"
            );
        };
        for (device, px_per_mm, (width, height), circle, offset) in [
            (GALAXY_S24, 16.378, (65.94, 142.88), 147.40, 122.83),
            (IPHONE_15, 18.110, (65.10, 141.14), 162.99, 135.83),
        ] {
            close(device.px_per_mm(), px_per_mm);
            let size = device.physical_size_mm();
            close(size.0, width);
            close(size.1, height);
            close(device.mm_to_px(9.0), circle);
            close(device.mm_to_px(7.5), offset);
        }
    }

    #[test]
    fn mm_and_px_round_trip() {
        for device in [GALAXY_S24, IPHONE_15] {
            for mm in [0.0, 0.5, 3.0, 9.0, 142.88] {
                let back = device.px_to_mm(device.mm_to_px(mm));
                assert!((back - mm).abs() < 1e-4, "{mm} came back as {back}");
            }
        }
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
