mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_playback_devices,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start SonaHop");
}
