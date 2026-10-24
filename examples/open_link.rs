//! Opens the link given as the argument in the default browser.
//!
//! `cargo run --example open_link --features open-link -- https://example.com/`

fn main() {
    let Some(url) = std::env::args().nth(1) else {
        eprintln!("usage: open_link <url>");
        std::process::exit(2);
    };
    match libdevice::link::open_link(&url) {
        Ok(()) => println!("opened"),
        Err(error) => {
            println!("{error:?}: {error}");
            std::process::exit(1);
        }
    }
}
