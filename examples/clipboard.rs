//! With no argument, prints the length in bytes of the clipboard's text
//! (never the text). With `--write <text>`, puts `<text>` on the clipboard
//! and prints `ok` or the error (never the text).
//!
//! `echo hello | pbcopy && cargo run --example clipboard --features clipboard`
//!
//! `cargo run --example clipboard --features clipboard -- --write 'héllo ✓'`

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => {
            match libdevice::clipboard::read_text() {
                Some(text) => println!("{}", text.len()),
                None => println!("no text"),
            }
            ExitCode::SUCCESS
        }
        [flag, text] if flag == "--write" => match libdevice::clipboard::write_text(text) {
            Ok(()) => {
                println!("ok");
                ExitCode::SUCCESS
            }
            Err(error) => {
                println!("{error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: clipboard [--write <text>]");
            ExitCode::from(2)
        }
    }
}
