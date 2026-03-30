use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../sidecar/distil_whisper.py");

    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let source = manifest_dir.join("../sidecar/distil_whisper.py");
    let bundled_copy = manifest_dir.join("resources/distil_whisper.py");

    if let Some(parent) = bundled_copy.parent() {
        fs::create_dir_all(parent).expect("failed to create bundled resources directory");
    }
    fs::copy(&source, &bundled_copy).expect("failed to copy distil_whisper.py into resources");

    tauri_build::build()
}
