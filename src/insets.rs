//! Inset, cutout and corner data, in physical pixels of the portrait display.
//!
//! The stored types follow Android's `WindowInsets.Type`, which is the finest
//! grained of the three platforms; iOS safe areas and CSS `env(safe-area-inset-*)`
//! are derived from them through [`InsetType`].

/// Distances from each display edge, in physical pixels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Insets {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

impl Insets {
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    /// The larger inset on each side, as Android's `Insets.max` and Compose's
    /// `WindowInsets.union` combine types.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        const fn max(a: u32, b: u32) -> u32 {
            if a > b { a } else { b }
        }
        Self {
            top: max(self.top, other.top),
            right: max(self.right, other.right),
            bottom: max(self.bottom, other.bottom),
            left: max(self.left, other.left),
        }
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.top == 0 && self.right == 0 && self.bottom == 0 && self.left == 0
    }

    /// Convert to logical units (dp, iOS points or CSS pixels) by dividing by
    /// the device pixel ratio.
    #[must_use]
    pub fn to_logical(self, pixel_ratio: f32) -> LogicalInsets {
        LogicalInsets {
            top: self.top as f32 / pixel_ratio,
            right: self.right as f32 / pixel_ratio,
            bottom: self.bottom as f32 / pixel_ratio,
            left: self.left as f32 / pixel_ratio,
        }
    }
}

/// Insets in logical units: dp on Android, points on iOS, CSS pixels on the web.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// The inset types stored for every device, matching Android's
/// `WindowInsets.Type`. Unset types are zero.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SystemInsets {
    pub status_bars: Insets,
    /// Android's navigation bar; the home indicator on iOS.
    pub navigation_bars: Insets,
    /// Freeform/desktop window caption; zero on phones.
    pub caption_bar: Insets,
    pub system_gestures: Insets,
    pub mandatory_system_gestures: Insets,
    pub tappable_element: Insets,
    /// The cutout safe insets. Taken from `devices.toml` when given there,
    /// otherwise derived from the cutout bounds and waterfall at build time.
    pub display_cutout: Insets,
    /// Curved-edge ("waterfall") display regions.
    pub waterfall: Insets,
}

/// An inset type to query with [`crate::DeviceSpec::insets`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InsetType {
    StatusBars,
    NavigationBars,
    CaptionBar,
    /// Status, navigation and caption bars.
    SystemBars,
    SystemGestures,
    MandatorySystemGestures,
    TappableElement,
    DisplayCutout,
    Waterfall,
    /// System bars and display cutout: where content can draw unobscured.
    /// Compose's `safeDrawing`.
    SafeDrawing,
    /// Every gesture region and the waterfall. Compose's `safeGestures`.
    SafeGestures,
    /// Safe drawing and safe gestures. Compose's `safeContent`.
    SafeContent,
    /// iOS `safeAreaInsets`: status bar, home indicator and sensor housing.
    IosSafeArea,
    /// CSS `env(safe-area-inset-*)` for a full-screen page with
    /// `viewport-fit=cover`, still in physical pixels; see
    /// [`crate::DeviceSpec::css_safe_area`] for CSS pixels.
    CssSafeArea,
}

impl SystemInsets {
    #[must_use]
    pub const fn get(&self, inset_type: InsetType) -> Insets {
        match inset_type {
            InsetType::StatusBars => self.status_bars,
            InsetType::NavigationBars => self.navigation_bars,
            InsetType::CaptionBar => self.caption_bar,
            InsetType::SystemBars => self
                .status_bars
                .union(self.navigation_bars)
                .union(self.caption_bar),
            InsetType::SystemGestures => self.system_gestures,
            InsetType::MandatorySystemGestures => self.mandatory_system_gestures,
            InsetType::TappableElement => self.tappable_element,
            InsetType::DisplayCutout => self.display_cutout,
            InsetType::Waterfall => self.waterfall,
            InsetType::SafeDrawing | InsetType::IosSafeArea | InsetType::CssSafeArea => {
                self.get(InsetType::SystemBars).union(self.display_cutout)
            }
            InsetType::SafeGestures => self
                .system_gestures
                .union(self.mandatory_system_gestures)
                .union(self.tappable_element)
                .union(self.waterfall),
            InsetType::SafeContent => self
                .get(InsetType::SafeDrawing)
                .union(self.get(InsetType::SafeGestures)),
        }
    }
}

/// The operating system a device runs, which decides its corner curve and
/// how a browser reports safe areas.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Platform {
    Android,
    Ios,
}

/// A display edge.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

/// An axis-aligned rectangle in physical display pixels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The outline of a cutout within its bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CutoutShape {
    /// A punch-hole camera; the bounds are its enclosing square.
    Circle,
    /// A pill with fully rounded ends, such as the iOS Dynamic Island.
    Capsule,
    /// A rectangle hanging from the edge with rounded outer corners, such as
    /// the iPhone X-14 notch or an Android teardrop.
    Notch { corner_radius: u32 },
    /// A plain rectangle.
    Rect,
    /// An SVG path in display pixels, as Android's
    /// `config_mainBuiltInDisplayCutout` takes; the bounds enclose it.
    Path(&'static str),
}

/// A region of the display taken by sensors or cameras.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cutout {
    pub edge: Edge,
    pub shape: CutoutShape,
    pub bounds: Rect,
}

/// How a display corner curves.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CornerShape {
    /// A circular arc of the given radius, as Android's `RoundedCorner`.
    #[default]
    Circular,
    /// Apple's continuous ("squircle") curve, which eases in over more than
    /// the nominal radius.
    Continuous,
}

/// Display corner radii in physical pixels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Corners {
    pub shape: CornerShape,
    pub top_left: u32,
    pub top_right: u32,
    pub bottom_right: u32,
    pub bottom_left: u32,
}
