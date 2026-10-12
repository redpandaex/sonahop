use crate::audio::{AudioService, AudioSnapshot};
use std::sync::Arc;
use tauri::State;
#[tauri::command]
pub fn get_audio_snapshot(service: State<'_, Arc<AudioService>>) -> AudioSnapshot {
    service.snapshot()
}
#[tauri::command]
pub async fn start_listening(
    service: State<'_, Arc<AudioService>>,
) -> Result<AudioSnapshot, String> {
    service.start_listening().await
}
#[tauri::command]
pub async fn request_microphone(
    service: State<'_, Arc<AudioService>>,
) -> Result<AudioSnapshot, String> {
    service.request_microphone().await
}
#[tauri::command]
pub async fn answer_microphone_request(
    peer_id: String,
    approve: bool,
    output_device_id: Option<String>,
    service: State<'_, Arc<AudioService>>,
) -> Result<AudioSnapshot, String> {
    service
        .answer_microphone(peer_id, approve, output_device_id)
        .await
}
#[tauri::command]
pub async fn stop_audio(service: State<'_, Arc<AudioService>>) -> Result<AudioSnapshot, String> {
    Ok(service.stop().await)
}
#[tauri::command]
pub fn set_audio_gain(
    gain: u8,
    service: State<'_, Arc<AudioService>>,
) -> Result<AudioSnapshot, String> {
    service.set_gain(gain)
}
