//! Writes the OpenAPI specs the frontend client and the docs are generated from:
//! `ui/static/openapi.json` (full, internal endpoints included) and
//! `ui/static/openapi-public.json` (internal endpoints removed).
//!
//! Run with: cargo run --bin generate-openapi
//! Or via: make generate-api-types

use scanopy::server::openapi::{full_spec, public_spec, write_spec};
use std::path::Path;
use utoipa::openapi::OpenApi;

fn main() {
    let static_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory")
        .join("ui/static");

    write(&full_spec(), &static_dir.join("openapi.json"), "full");
    write(
        &public_spec(),
        &static_dir.join("openapi-public.json"),
        "public",
    );
}

fn write(spec: &OpenApi, path: &Path, label: &str) {
    write_spec(spec, path).unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));

    println!("✅ Generated openapi.json ({label}) at {}", path.display());
    println!("   Paths: {}", spec.paths.paths.len());
    if let Some(components) = &spec.components {
        println!("   Schemas: {}", components.schemas.len());
    }
}
