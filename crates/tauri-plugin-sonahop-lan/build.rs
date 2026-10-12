const COMMANDS: &[&str] = &["ensure_permission", "prepare_audio", "list_capture_apps"];
fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
