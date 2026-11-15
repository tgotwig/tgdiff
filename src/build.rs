/// Runs the Tauri build-time codegen (config is read from the crate root `tauri.conf.json`).
fn main() {
    tauri_build::build();
}
