//! The instance and frame headers every instanced renderer shares.
//!
//! Every renderer's per-instance record starts with an [`InstanceHeader`] and
//! every frame uniform with a [`FrameHeader`], so anything that places or
//! animates instances - a prop-file reader, a previewer, later the ECS - writes
//! the header without knowing which renderer will draw it.
//!
//! The GPU structs hold plain arrays rather than glam types, so every offset is
//! explicit and matches WGSL whatever SIMD alignment glam uses on a target.
//! glam is used at the edges, through `to_array` and `from_array`.
//!
//! Conventions are glam's, and therefore Bevy's: right-handed, `+y` up, `-z`
//! forward; rotations are active; quaternions are stored `[x, y, z, w]`.

use std::fmt;

use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

/// The instance is not drawn.
pub const HEADER_HIDDEN: u32 = 1 << 0;
/// The instance is selected.
pub const HEADER_SELECTED: u32 = 1 << 1;
/// The instance is highlighted, for example under the pointer.
pub const HEADER_HIGHLIGHT: u32 = 1 << 2;
/// Bits reserved for later common use. No renderer may use any header flag
/// bit for its own purposes; its own flags live in its record body.
pub const HEADER_RESERVED: u32 = 0xFFFF_0000;

/// WGSL declarations of both headers, their flag bits and their helpers:
/// `quat_rotate`, `quat_conjugate`, `header_to_local`, `header_to_world`,
/// `header_dir_to_local`, `header_dir_to_world` and `header_time`. A renderer
/// joins this ahead of its own source. Validated by naga at build time.
pub const HEADER_WGSL: &str = include_str!("instance.wgsl");

/// 48 bytes at the start of every instance record: where it is, how big,
/// which way round, and its clock.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct InstanceHeader {
    /// World position, metres.
    pub position: [f32; 3],
    /// Uniform scale; 1.0 is the authored size. Uniform only: a non-uniform
    /// scale leaves a signed distance field no longer a distance.
    pub scale: f32,
    /// Unit quaternion `[x, y, z, w]`, rotating the instance's own frame into
    /// the world's.
    pub rotation: [f32; 4],
    /// Seconds added to the frame's time for this instance.
    pub time_offset: f32,
    /// 1.0 normal, 0.0 frozen.
    pub time_scale: f32,
    /// Stable identity, assigned by whoever places the instance. Not an ECS
    /// entity's bits: those are recycled, and this feeds seeds and selection.
    pub id: u32,
    /// The common `HEADER_*` bits.
    pub flags: u32,
}

/// 16 bytes at the start of every renderer's frame uniform.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct FrameHeader {
    /// Seconds. From Bevy, `elapsed_secs_wrapped()`, so it stays precise over
    /// long sessions.
    pub time: f32,
    /// Seconds since the last frame.
    pub dt: f32,
    /// Frame counter, wrapping.
    pub frame: u32,
    pub _pad: u32,
}

impl FrameHeader {
    #[must_use]
    pub const fn new(time: f32, dt: f32, frame: u32) -> Self {
        Self {
            time,
            dt,
            frame,
            _pad: 0,
        }
    }
}

impl InstanceHeader {
    /// At the origin, at its authored size, unturned, on the frame's clock,
    /// with id 0 and no flags.
    pub const IDENTITY: Self = Self {
        position: [0.0; 3],
        scale: 1.0,
        rotation: [0.0, 0.0, 0.0, 1.0],
        time_offset: 0.0,
        time_scale: 1.0,
        id: 0,
        flags: 0,
    };

    /// Placed at `position`, turned by the quaternion `rotation` (`[x, y, z,
    /// w]`, normalised here) and scaled by `scale`.
    #[must_use]
    pub fn from_parts(position: [f32; 3], rotation: [f32; 4], scale: f32) -> Self {
        Self::from_glam(
            Vec3::from_array(position),
            Quat::from_array(rotation),
            scale,
        )
    }

    /// As [`InstanceHeader::from_parts`], from glam types.
    #[must_use]
    pub fn from_glam(position: Vec3, rotation: Quat, scale: f32) -> Self {
        Self {
            position: position.to_array(),
            scale,
            rotation: canonical(rotation).to_array(),
            ..Self::IDENTITY
        }
    }

    /// Placed at `position` and turned by `yaw` radians about `+y`, at the
    /// authored size: `Quat::from_rotation_y(yaw)`, which takes `+x` to
    /// `(cos yaw, 0, -sin yaw)`.
    #[must_use]
    pub fn from_yaw(position: [f32; 3], yaw: f32) -> Self {
        Self::from_glam(Vec3::from_array(position), Quat::from_rotation_y(yaw), 1.0)
    }

    /// The rotation, normalised, with `w >= 0` so the same turn always reads
    /// the same.
    #[must_use]
    pub fn rotation(&self) -> Quat {
        canonical(Quat::from_array(self.rotation))
    }

    #[must_use]
    pub fn position(&self) -> Vec3 {
        Vec3::from_array(self.position)
    }

    /// A point in the instance's own frame placed in the world: scaled,
    /// rotated, then moved.
    #[must_use]
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.position() + self.rotation() * (local * self.scale)
    }

    /// A world point in the instance's own frame: the inverse of
    /// [`InstanceHeader::to_world`].
    #[must_use]
    pub fn to_local(&self, world: Vec3) -> Vec3 {
        self.rotation().inverse() * ((world - self.position()) / self.scale)
    }

    /// The same header on its own clock: `offset` seconds added to the frame's
    /// time after it is multiplied by `scale`.
    #[must_use]
    pub const fn with_time(self, offset: f32, scale: f32) -> Self {
        Self {
            time_offset: offset,
            time_scale: scale,
            ..self
        }
    }

    #[must_use]
    pub const fn with_id(self, id: u32) -> Self {
        Self { id, ..self }
    }

    #[must_use]
    pub const fn with_flags(self, flags: u32) -> Self {
        Self { flags, ..self }
    }
}

impl Default for InstanceHeader {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// `q` normalised, with its sign chosen so `w >= 0`: `q` and `-q` are the same
/// rotation, and comparisons need one of them.
fn canonical(q: Quat) -> Quat {
    let q = q.normalize();
    if q.w < 0.0 { -q } else { q }
}

/// A scale whose three components differ by more than 0.1%.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct NonUniformScale {
    pub scale: [f32; 3],
}

impl NonUniformScale {
    /// The largest component, which a caller should fall back to after
    /// logging - never a silent average.
    #[must_use]
    pub fn largest(&self) -> f32 {
        self.scale[0].max(self.scale[1]).max(self.scale[2])
    }
}

impl fmt::Display for NonUniformScale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [x, y, z] = self.scale;
        write!(
            f,
            "scale ({x}, {y}, {z}) is not uniform: a signed distance field takes one scale"
        )
    }
}

impl std::error::Error for NonUniformScale {}

/// How far apart a scale's components may be, as a share of the largest,
/// before it is no longer uniform.
const UNIFORM_TOLERANCE: f32 = 0.001;

/// `scale`'s single value, or `Err` if its three components differ by more than
/// 0.1%: for converting a Bevy `Transform`'s scale.
pub fn uniform_scale(scale: [f32; 3]) -> Result<f32, NonUniformScale> {
    let [x, y, z] = scale;
    let largest = x.max(y).max(z);
    let smallest = x.min(y).min(z);
    if !(largest - smallest).is_finite() || largest - smallest > UNIFORM_TOLERANCE * largest.abs() {
        return Err(NonUniformScale { scale });
    }
    Ok((x + y + z) / 3.0)
}

#[cfg(test)]
mod tests {
    use super::{FrameHeader, InstanceHeader, uniform_scale};
    use glam::{Quat, Vec3};
    use std::mem::{offset_of, size_of};

    #[test]
    fn instance_header_matches_the_shader_layout() {
        assert_eq!(size_of::<InstanceHeader>(), 48);
        assert_eq!(offset_of!(InstanceHeader, position), 0);
        assert_eq!(offset_of!(InstanceHeader, scale), 12);
        assert_eq!(offset_of!(InstanceHeader, rotation), 16);
        assert_eq!(offset_of!(InstanceHeader, time_offset), 32);
        assert_eq!(offset_of!(InstanceHeader, time_scale), 36);
        assert_eq!(offset_of!(InstanceHeader, id), 40);
        assert_eq!(offset_of!(InstanceHeader, flags), 44);
    }

    #[test]
    fn frame_header_matches_the_shader_layout() {
        assert_eq!(size_of::<FrameHeader>(), 16);
        assert_eq!(offset_of!(FrameHeader, time), 0);
        assert_eq!(offset_of!(FrameHeader, dt), 4);
        assert_eq!(offset_of!(FrameHeader, frame), 8);
        assert_eq!(offset_of!(FrameHeader, _pad), 12);
    }

    #[test]
    fn a_yaw_takes_x_where_from_rotation_y_does() {
        for a in [0.0f32, 0.3, 1.0, 2.5, -1.2] {
            let h = InstanceHeader::from_yaw([0.0; 3], a);
            let x = h.to_world(Vec3::X);
            let want = Vec3::new(a.cos(), 0.0, -a.sin());
            assert!((x - want).length() < 1e-6, "{a}: {x} vs {want}");
        }
    }

    /// A small deterministic generator, so the round trip is tested over many
    /// headers without a dependency.
    struct Lcg(u64);
    impl Lcg {
        fn unit(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }
        fn range(&mut self, lo: f32, hi: f32) -> f32 {
            lo + (hi - lo) * self.unit()
        }
    }

    #[test]
    fn to_local_undoes_to_world() {
        let mut r = Lcg(7);
        for _ in 0..500 {
            let rotation = [
                r.range(-1.0, 1.0),
                r.range(-1.0, 1.0),
                r.range(-1.0, 1.0),
                r.range(-1.0, 1.0),
            ];
            let h = InstanceHeader::from_parts(
                [r.range(-5.0, 5.0), r.range(-5.0, 5.0), r.range(-5.0, 5.0)],
                rotation,
                r.range(0.25, 4.0),
            );
            let v = Vec3::new(r.range(-1.0, 1.0), r.range(-1.0, 1.0), r.range(-1.0, 1.0));
            let back = h.to_local(h.to_world(v));
            assert!((back - v).length() < 1e-5, "{h:?}: {v} -> {back}");
        }
    }

    #[test]
    fn uniform_scale_accepts_uniform_and_rejects_the_rest() {
        assert_eq!(uniform_scale([2.0, 2.0, 2.0]), Ok(2.0));
        assert!(uniform_scale([1.0, 2.0, 1.0]).is_err());
        assert!(uniform_scale([1.0, 1.0005, 1.0]).is_ok());
        assert_eq!(uniform_scale([1.0, 2.0, 1.0]).unwrap_err().largest(), 2.0);
    }

    #[test]
    fn rotation_is_normalised_with_w_non_negative() {
        let h = InstanceHeader {
            rotation: [0.0, 2.0, 0.0, -2.0],
            ..InstanceHeader::IDENTITY
        };
        let q = h.rotation();
        assert!((q.length() - 1.0).abs() < 1e-6);
        assert!(q.w >= 0.0);
        let same = Quat::from_array([0.0, -2.0, 0.0, 2.0]).normalize();
        assert!(q.abs_diff_eq(same, 1e-6));
    }

    #[test]
    fn identity_is_the_untouched_header() {
        let h = InstanceHeader::IDENTITY;
        let v = Vec3::new(0.3, -0.2, 0.9);
        assert_eq!(h.to_world(v), v);
        assert_eq!(h.with_id(9).id, 9);
        let t = h.with_time(1.5, 0.0);
        assert_eq!((t.time_offset, t.time_scale), (1.5, 0.0));
    }

    #[test]
    fn the_wgsl_declares_both_headers_and_their_helpers() {
        for name in [
            "struct InstanceHeader",
            "struct FrameHeader",
            "fn quat_rotate",
            "fn quat_conjugate",
            "fn header_to_local",
            "fn header_to_world",
            "fn header_time",
        ] {
            assert!(super::HEADER_WGSL.contains(name), "{name}");
        }
    }
}
