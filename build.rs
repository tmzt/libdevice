//! Compile `devices.toml` into const `DeviceSpec` data.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

const SOURCE: &str = "devices.toml";

fn main() {
    println!("cargo::rerun-if-changed={SOURCE}");

    let text = fs::read_to_string(SOURCE).unwrap_or_else(|error| panic!("{SOURCE}: {error}"));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|error| panic!("{SOURCE}: {error}"));
    assert!(!table.is_empty(), "{SOURCE}: no devices defined");

    let mut out = String::new();
    let mut names = Vec::new();
    for (key, value) in &table {
        let device = value
            .as_table()
            .unwrap_or_else(|| panic!("{SOURCE}: `{key}` must be a table"));
        assert!(
            !key.is_empty()
                && !key.starts_with(|c: char| c.is_ascii_digit())
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{SOURCE}: device key `{key}` must be snake_case"
        );
        let name = string(device, key, "name");
        let width = dimension(device, key, "width");
        let height = dimension(device, key, "height");
        let ident = key.to_ascii_uppercase();

        writeln!(
            out,
            "/// {name}.\n\
             pub const {ident}: DeviceSpec = DeviceSpec {{\n    \
                 id: {key:?},\n    \
                 name: {name:?},\n    \
                 geometry: PortraitGeometry {{ width: {width}, height: {height} }},\n\
             }};\n"
        )
        .unwrap();
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

fn string<'a>(device: &'a toml::Table, key: &str, field: &str) -> &'a str {
    device
        .get(field)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("{SOURCE}: `{key}.{field}` must be a string"))
}

fn dimension(device: &toml::Table, key: &str, field: &str) -> u32 {
    device
        .get(field)
        .and_then(toml::Value::as_integer)
        .and_then(|value| u32::try_from(value).ok())
        .filter(|&value| value > 0)
        .unwrap_or_else(|| panic!("{SOURCE}: `{key}.{field}` must be a positive 32-bit integer"))
}
