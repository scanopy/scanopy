//! Generates metadata fixture JSON files for the frontend.
//!
//! Run with: cargo run --bin generate-fixtures
//! Or via: make generate-fixtures

use scanopy::server::shared::fixtures::generate_ui_data_fixtures;
use std::path::PathBuf;

fn main() {
    let output_dir = parse_output_dir();
    generate_ui_data_fixtures(&output_dir);
}

fn parse_output_dir() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    for i in 1..args.len() {
        if args[i] == "--output-dir" {
            if let Some(dir) = args.get(i + 1) {
                return PathBuf::from(dir);
            }
            eprintln!("Error: --output-dir requires a path argument");
            std::process::exit(1);
        }
    }
    PathBuf::from("../ui/src/lib/data")
}
