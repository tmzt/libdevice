//! Prints the data and cache directories for the app id given as the
//! argument. Nothing is created.
//!
//! `cargo run --example dirs -- dev.tmzt.handlr`

fn main() {
    let Some(app_id) = std::env::args().nth(1) else {
        eprintln!("usage: dirs <app id>");
        std::process::exit(2);
    };
    println!("data:  {:?}", libdevice::dirs::data_dir(&app_id));
    println!("cache: {:?}", libdevice::dirs::cache_dir(&app_id));
}
