fn main() {
    println!("cargo:rerun-if-changed=../sidecar/distil_whisper.py");
    println!("cargo:rerun-if-changed=../sidecar/python-env");

    tauri_build::build()
}
