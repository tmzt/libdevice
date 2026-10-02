// The instance and frame headers every instanced renderer shares.
//
// Mirrors src/instance.rs field for field; that file's layout tests pin every
// offset. A renderer joins this source ahead of its own, so its per-instance
// record can begin with an `InstanceHeader` and its frame uniform with a
// `FrameHeader`.

// 48 bytes: where an instance is, how big, which way round, and its clock.
struct InstanceHeader {
    // World position, metres.
    position: vec3<f32>,
    // Uniform scale; 1.0 is the authored size.
    scale: f32,
    // Unit quaternion [x, y, z, w]: rotates the instance's own frame into
    // the world's.
    rotation: vec4<f32>,
    // Seconds added to the frame's time for this instance.
    time_offset: f32,
    // 1.0 normal, 0.0 frozen.
    time_scale: f32,
    // Stable identity, assigned by whoever places the instance.
    id: u32,
    // Common bits only; a renderer's own flags live in its record body.
    flags: u32,
};

// 16 bytes, at the start of every renderer's frame uniform.
struct FrameHeader {
    // Seconds; wrapped by the caller so it stays precise over long sessions.
    time: f32,
    // Seconds since the last frame.
    dt: f32,
    // Frame counter, wrapping.
    frame: u32,
    pad: u32,
};

// The common header flags, matching src/instance.rs. Bits 16..31 are reserved
// for later common use; no renderer may use these bits for its own purposes.
const HEADER_HIDDEN: u32 = 1u;
const HEADER_SELECTED: u32 = 2u;
const HEADER_HIGHLIGHT: u32 = 4u;

// Rotate `v` by the unit quaternion `q`.
fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

// The inverse of a unit quaternion.
fn quat_conjugate(q: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(-q.xyz, q.w);
}

// A world point in the instance's own frame: `(world - position) / scale`,
// rotated by the conjugate.
//
// A distance measured in this frame is in the instance's own units. Multiply
// it by `h.scale` on the way back out, or a scaled instance's field is no
// longer a distance and the march oversteps or crawls.
fn header_to_local(h: InstanceHeader, world: vec3<f32>) -> vec3<f32> {
    return quat_rotate(quat_conjugate(h.rotation), (world - h.position) / h.scale);
}

// A direction in the world turned into the instance's own frame, unscaled.
fn header_dir_to_local(h: InstanceHeader, dir: vec3<f32>) -> vec3<f32> {
    return quat_rotate(quat_conjugate(h.rotation), dir);
}

// A point in the instance's own frame placed in the world: scaled, rotated,
// then moved. Lengths in the instance's frame come out `h.scale` times longer.
fn header_to_world(h: InstanceHeader, local: vec3<f32>) -> vec3<f32> {
    return h.position + quat_rotate(h.rotation, local * h.scale);
}

// A direction in the instance's own frame turned into the world, unscaled.
fn header_dir_to_world(h: InstanceHeader, dir: vec3<f32>) -> vec3<f32> {
    return quat_rotate(h.rotation, dir);
}

// This instance's clock: the frame's time, slowed or frozen, then offset.
fn header_time(h: InstanceHeader, frame: FrameHeader) -> f32 {
    return frame.time * h.time_scale + h.time_offset;
}
