#![allow(dead_code)]

#[path = "src/scripting/api_catalog.rs"]
mod api_catalog;

fn main() {
    println!("cargo:rerun-if-changed=src/scripting/api_catalog.rs");
    println!("cargo:rerun-if-changed=build.rs");
    std::fs::write("API.md", api_catalog::render_markdown())
        .expect("write generated script API reference");
}
