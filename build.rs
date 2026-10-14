//! Compile `devices.toml` into const `DeviceSpec` data.
//!
//! The file is validated here so a typo fails the build instead of producing
//! a device with silently zeroed insets.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use toml::{Table, Value};

const SOURCE: &str = "devices.toml";

const DEVICE_FIELDS: &[&str] = &[
    "name",
    "platform",
    "width",
    "height",
    "pixel_ratio",
    "insets",
    "cutouts",
    "corners",
];
const INSET_TYPES: &[&str] = &[
    "status_bars",
    "navigation_bars",
    "caption_bar",
    "system_gestures",
    "mandatory_system_gestures",
    "tappable_element",
    "display_cutout",
    "waterfall",
];
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
const CORNERS: [&str; 4] = ["top_left", "top_right", "bottom_right", "bottom_left"];

fn main() {
    println!("cargo::rerun-if-changed={SOURCE}");

    let text = fs::read_to_string(SOURCE).unwrap_or_else(|error| fail(&error.to_string()));
    let table: Table = text
        .parse()
        .unwrap_or_else(|error: toml::de::Error| fail(&error.to_string()));
    if table.is_empty() {
        fail("no devices defined");
    }

    let mut out = String::new();
    let mut names = Vec::new();
    for (key, value) in &table {
        let ident = device(&mut out, key, value);
        names.push(ident);
    }
    writeln!(
        out,
        "/// Every device in `devices.toml`, in file order.\n\
         pub const DEVICES: &[DeviceSpec] = &[{}];\n\n\
         /// The first device in `devices.toml`.\n\
         pub const DEFAULT_DEVICE: DeviceSpec = {};",
        names.join(", "),
        names[0]
    )
    .unwrap();

    let path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("devices.rs");
    fs::write(path, out).unwrap();
}

/// Emit one device const and return its identifier.
fn device(out: &mut String, key: &str, value: &Value) -> String {
    if key.is_empty()
        || key.starts_with(|c: char| c.is_ascii_digit())
        || !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        fail(&format!("device key `{key}` must be snake_case"));
    }
    let device = table(value, key);
    known(device, key, DEVICE_FIELDS);

    let name = string(device, key, "name");
    let platform = match string(device, key, "platform") {
        "android" => "Android",
        "ios" => "Ios",
        other => fail(&format!(
            "`{key}.platform` is `{other}`, expected `android` or `ios`"
        )),
    };
    let width = positive(device, key, "width");
    let height = positive(device, key, "height");
    let pixel_ratio = device
        .get("pixel_ratio")
        .and_then(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|i| i as f64))
        })
        .filter(|ratio| ratio.is_finite() && *ratio > 0.0)
        .unwrap_or_else(|| fail(&format!("`{key}.pixel_ratio` must be a positive number")));

    let cutouts: Vec<Cutout> = match device.get("cutouts") {
        None => Vec::new(),
        Some(value) => value
            .as_array()
            .unwrap_or_else(|| fail(&format!("`{key}.cutouts` must be an array of tables")))
            .iter()
            .enumerate()
            .map(|(index, value)| cutout(value, &format!("{key}.cutouts[{index}]"), width, height))
            .collect(),
    };

    let insets = device.get("insets").map_or_else(Table::new, |value| {
        table(value, &format!("{key}.insets")).clone()
    });
    known(&insets, &format!("{key}.insets"), INSET_TYPES);
    let mut resolved = Vec::new();
    for inset_type in INSET_TYPES {
        let context = format!("{key}.insets.{inset_type}");
        let sides = match insets.get(*inset_type) {
            Some(value) => sides(value, &context),
            // Without an explicit figure, the cutout insets reach past every
            // cutout and the waterfall, as Android computes them.
            None if *inset_type == "display_cutout" => {
                let waterfall = insets.get("waterfall").map_or([0; 4], |value| {
                    sides(value, &format!("{key}.insets.waterfall"))
                });
                let mut sides = waterfall;
                for cutout in &cutouts {
                    let (side, extent) = match cutout.edge {
                        "Top" => (0, cutout.y + cutout.height),
                        "Right" => (1, width - cutout.x),
                        "Bottom" => (2, height - cutout.y),
                        _ => (3, cutout.x + cutout.width),
                    };
                    sides[side] = sides[side].max(extent);
                }
                sides
            }
            None => [0; 4],
        };
        resolved.push((inset_type, sides));
    }

    let corners = corners(device.get("corners"), key, platform);

    let ident = key.to_ascii_uppercase();
    writeln!(
        out,
        "/// {name}.\npub const {ident}: DeviceSpec = DeviceSpec {{\n    \
         id: {key:?},\n    \
         name: {name:?},\n    \
         platform: Platform::{platform},\n    \
         geometry: PortraitGeometry {{ width: {width}, height: {height} }},\n    \
         pixel_ratio: {pixel_ratio:?},\n    \
         insets: SystemInsets {{"
    )
    .unwrap();
    for (inset_type, [top, right, bottom, left]) in resolved {
        writeln!(
            out,
            "        {inset_type}: Insets {{ top: {top}, right: {right}, bottom: {bottom}, left: {left} }},"
        )
        .unwrap();
    }
    writeln!(out, "    }},\n    cutouts: &[").unwrap();
    for cutout in &cutouts {
        writeln!(
            out,
            "        Cutout {{ edge: Edge::{}, shape: {}, bounds: Rect {{ x: {}, y: {}, width: {}, height: {} }} }},",
            cutout.edge, cutout.shape, cutout.x, cutout.y, cutout.width, cutout.height
        )
        .unwrap();
    }
    writeln!(out, "    ],\n    corners: {corners},\n}};\n").unwrap();
    ident
}

struct Cutout {
    edge: &'static str,
    shape: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn cutout(value: &Value, context: &str, display_width: u32, display_height: u32) -> Cutout {
    let cutout = table(value, context);
    known(
        cutout,
        context,
        &[
            "edge",
            "shape",
            "x",
            "y",
            "width",
            "height",
            "corner_radius",
            "path",
        ],
    );
    let edge = match string(cutout, context, "edge") {
        "top" => "Top",
        "right" => "Right",
        "bottom" => "Bottom",
        "left" => "Left",
        other => fail(&format!("`{context}.edge` is `{other}`, expected a side")),
    };
    let x = unsigned(cutout, context, "x");
    let y = unsigned(cutout, context, "y");
    let width = positive(cutout, context, "width");
    let height = positive(cutout, context, "height");
    if x + width > display_width || y + height > display_height {
        fail(&format!("`{context}` extends past the display"));
    }
    let shape_name = string(cutout, context, "shape");
    let only = |field: &str| {
        for extra in ["corner_radius", "path"] {
            if extra != field && cutout.contains_key(extra) {
                fail(&format!(
                    "`{context}.{extra}` does not apply to a {shape_name} cutout"
                ));
            }
        }
    };
    let shape = match shape_name {
        "circle" | "capsule" | "rect" => {
            only("");
            match shape_name {
                "circle" => "CutoutShape::Circle".to_owned(),
                "capsule" => "CutoutShape::Capsule".to_owned(),
                _ => "CutoutShape::Rect".to_owned(),
            }
        }
        "notch" => {
            only("corner_radius");
            let radius = unsigned(cutout, context, "corner_radius");
            format!("CutoutShape::Notch {{ corner_radius: {radius} }}")
        }
        "path" => {
            only("path");
            format!("CutoutShape::Path({:?})", string(cutout, context, "path"))
        }
        other => fail(&format!(
            "`{context}.shape` is `{other}`, expected circle, capsule, notch, rect or path"
        )),
    };
    Cutout {
        edge,
        shape,
        x,
        y,
        width,
        height,
    }
}

/// `corners` is either one radius or a table with `radius`, per-corner
/// overrides and `shape`. The shape defaults to the platform's curve.
fn corners(value: Option<&Value>, key: &str, platform: &str) -> String {
    let context = format!("{key}.corners");
    let default_shape = if platform == "Ios" {
        "Continuous"
    } else {
        "Circular"
    };
    let (shape, radii) = match value {
        None => (default_shape, [0; 4]),
        Some(Value::Integer(_)) => {
            let radius = unsigned_value(value.unwrap(), &context);
            (default_shape, [radius; 4])
        }
        Some(value) => {
            let corners = table(value, &context);
            known(
                corners,
                &context,
                &[
                    "shape",
                    "radius",
                    "top_left",
                    "top_right",
                    "bottom_right",
                    "bottom_left",
                ],
            );
            let shape = match corners
                .get("shape")
                .map(|_| string(corners, &context, "shape"))
            {
                None => default_shape,
                Some("circular") => "Circular",
                Some("continuous") => "Continuous",
                Some(other) => fail(&format!(
                    "`{context}.shape` is `{other}`, expected circular or continuous"
                )),
            };
            let radius = if corners.contains_key("radius") {
                unsigned(corners, &context, "radius")
            } else {
                0
            };
            let radii = CORNERS.map(|corner| {
                if corners.contains_key(corner) {
                    unsigned(corners, &context, corner)
                } else {
                    radius
                }
            });
            (shape, radii)
        }
    };
    let [top_left, top_right, bottom_right, bottom_left] = radii;
    format!(
        "Corners {{ shape: CornerShape::{shape}, top_left: {top_left}, top_right: {top_right}, \
         bottom_right: {bottom_right}, bottom_left: {bottom_left} }}"
    )
}

/// An inset table such as `{ top = 110 }`; missing sides are zero.
fn sides(value: &Value, context: &str) -> [u32; 4] {
    let sides = table(value, context);
    known(sides, context, &SIDES);
    SIDES.map(|side| {
        if sides.contains_key(side) {
            unsigned(sides, context, side)
        } else {
            0
        }
    })
}

fn table<'a>(value: &'a Value, context: &str) -> &'a Table {
    value
        .as_table()
        .unwrap_or_else(|| fail(&format!("`{context}` must be a table")))
}

fn known(table: &Table, context: &str, fields: &[&str]) {
    for key in table.keys() {
        if !fields.contains(&key.as_str()) {
            fail(&format!(
                "unknown field `{context}.{key}`; expected one of {}",
                fields.join(", ")
            ));
        }
    }
}

fn string<'a>(table: &'a Table, context: &str, field: &str) -> &'a str {
    table
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| fail(&format!("`{context}.{field}` must be a string")))
}

fn unsigned(table: &Table, context: &str, field: &str) -> u32 {
    let value = table
        .get(field)
        .unwrap_or_else(|| fail(&format!("`{context}.{field}` is required")));
    unsigned_value(value, &format!("{context}.{field}"))
}

fn unsigned_value(value: &Value, context: &str) -> u32 {
    value
        .as_integer()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_else(|| {
            fail(&format!(
                "`{context}` must be a non-negative 32-bit integer"
            ))
        })
}

fn positive(table: &Table, context: &str, field: &str) -> u32 {
    let value = unsigned(table, context, field);
    if value == 0 {
        fail(&format!("`{context}.{field}` must be positive"));
    }
    value
}

fn fail(message: &str) -> ! {
    panic!("{SOURCE}: {message}")
}
