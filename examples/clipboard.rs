//! Prints the length in bytes of the clipboard's text (never the text).
//!
//! `echo hello | pbcopy && cargo run --example clipboard --features clipboard`

fn main() {
    match libdevice::clipboard::read_text() {
        Some(text) => println!("{}", text.len()),
        None => println!("no text"),
    }
}
