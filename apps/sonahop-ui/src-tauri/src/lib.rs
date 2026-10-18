mod audio;
mod audio_commands;
mod commands;
mod diagnostics_commands;
mod link_commands;
mod session_commands;
use audio::AudioService;
use sonahop_link::{LinkRole, LinkService};
use std::sync::Arc;
use tauri::Manager;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let role = if cfg!(target_os = "android") {
        LinkRole::Client
    } else {
        LinkRole::Host
    };
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    #[cfg(target_os = "windows")]
    let builder = builder.plugin(tauri_plugin_autostart::Builder::new().build());
    let app = builder
        .plugin(tauri_plugin_sonahop_lan::init())
        .manage(Arc::new(LinkService::new(role)))
        .manage(session_commands::OperationGate::default())
        .setup(move |app| {
            let link = app.state::<Arc<LinkService>>().inner().clone();
            let log_result = app.path().app_log_dir().map(|dir| dir.join("sonahop-link.jsonl"));
            match log_result {
                Ok(path) => if let Err(error)=link.configure_logging(path) { link.log_event("log-unavailable",serde_json::json!({"reason":error})); },
                Err(error) => link.log_event("log-unavailable",serde_json::json!({"reason":error.to_string()})),
            }
            link.log_event("app-start",serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS}));
            let audio = Arc::new(AudioService::new(app.handle().clone(), link, role));
            audio.start_events();
            app.manage(audio);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            diagnostics_commands::get_diagnostics,
            commands::list_playback_devices,
            commands::list_capture_devices,
            session_commands::switch_connection_role,
            link_commands::get_link_snapshot,
            link_commands::start_link,
            link_commands::stop_link,
            link_commands::connect_link,
            link_commands::disconnect_link_peer,
            audio_commands::get_audio_snapshot,
            audio_commands::start_listening,
            audio_commands::request_microphone,
            audio_commands::answer_microphone_request,
            audio_commands::stop_audio,
            audio_commands::stop_audio_lane,
            audio_commands::set_audio_gain,
        ])
        .build(tauri::generate_context!())
        .expect("failed to start SonaHop");
    app.run(|handle, event| {
        if matches!(
            event,
            tauri::RunEvent::Exit
                | tauri::RunEvent::WindowEvent {
                    event: tauri::WindowEvent::Destroyed,
                    ..
                }
        ) {
            let cleanup_handle = handle.clone();
            let cleanup = async move {
                let handle = cleanup_handle;
                if let Some(audio) = handle.try_state::<Arc<AudioService>>() {
                    audio.stop().await;
                }
                if let Some(link) = handle.try_state::<Arc<LinkService>>() {
                    link.stop().await;
                }
            };
            #[cfg(target_os = "android")]
            tauri::async_runtime::spawn(cleanup);
            #[cfg(not(target_os = "android"))]
            tauri::async_runtime::block_on(cleanup);
        }
    });
}
