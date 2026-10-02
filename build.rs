//! Validates the shared header WGSL with naga at build time, the way
//! libbarstool validates its shaders, so a renderer that includes it is never
//! the first to find it broken.

const SHADER: &str = "src/instance.wgsl";

fn main() {
    println!("cargo:rerun-if-changed={SHADER}");
    let source = std::fs::read_to_string(SHADER)
        .unwrap_or_else(|error| panic!("{SHADER} must be readable: {error}"));
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
        panic!(
            "{SHADER}: WGSL syntax error: {}",
            error.emit_to_string(&source)
        )
    });
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    if let Err(error) = validator.validate(&module) {
        panic!(
            "{SHADER}: WGSL validation error: {}",
            error.emit_to_string(&source)
        );
    }
}
