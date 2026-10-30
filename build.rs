#![allow(dead_code)]

#[path = "src/api/rhai/catalog.rs"]
mod api_catalog;

fn main() {
    println!("cargo:rerun-if-changed=src/api/rhai/catalog.rs");
    println!("cargo:rerun-if-changed=build.rs");
    std::fs::write("docs/rhai-api.md", api_catalog::render_markdown())
        .expect("write generated script API reference");
}
