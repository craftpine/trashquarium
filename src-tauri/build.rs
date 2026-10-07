fn main() {
    // The Windows exe icon is compiled into a resource by the build script; rebuild it when the icons change.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
