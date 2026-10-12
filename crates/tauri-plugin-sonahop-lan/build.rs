const COMMANDS: &[&str] = &["ensure_permission", "prepare_audio"];
fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
