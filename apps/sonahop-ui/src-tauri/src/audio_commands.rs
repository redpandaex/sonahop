use crate::audio::{AudioLane, AudioService, AudioSnapshot, AudioSource};
use crate::session_commands::OperationGate;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_audio_snapshot(service: State<'_, Arc<AudioService>>) -> AudioSnapshot {
    service.snapshot()
}
#[tauri::command]
pub async fn start_listening(
    source: Option<AudioSource>,
    input_device_id: Option<String>,
    output_device_id: Option<String>,
    service: State<'_, Arc<AudioService>>,
    gate: State<'_, OperationGate>,
) -> Result<AudioSnapshot, String> {
    let _guard = gate.0.lock().await;
    service
        .start_listening(source, input_device_id, output_device_id)
        .await
}
#[tauri::command]
pub async fn request_microphone(
    input_device_id: Option<String>,
    service: State<'_, Arc<AudioService>>,
    gate: State<'_, OperationGate>,
) -> Result<AudioSnapshot, String> {
    let _guard = gate.0.lock().await;
    service.request_microphone(input_device_id).await
}
#[tauri::command]
pub async fn answer_microphone_request(
    peer_id: String,
    request_id: String,
    approve: bool,
    output_device_id: Option<String>,
    service: State<'_, Arc<AudioService>>,
    gate: State<'_, OperationGate>,
) -> Result<AudioSnapshot, String> {
    let _guard = gate.0.lock().await;
    service
        .answer_microphone(peer_id, request_id, approve, output_device_id)
        .await
}
#[tauri::command]
pub async fn stop_audio(
    service: State<'_, Arc<AudioService>>,
    gate: State<'_, OperationGate>,
) -> Result<AudioSnapshot, String> {
    let _guard = gate.0.lock().await;
    Ok(service.stop().await)
}
#[tauri::command]
pub async fn set_audio_gain(
    gain: u8,
    service: State<'_, Arc<AudioService>>,
    gate: State<'_, OperationGate>,
) -> Result<AudioSnapshot, String> {
    let _guard = gate.0.lock().await;
    service.set_gain(gain)
}
#[tauri::command]
pub async fn stop_audio_lane(
    lane: AudioLane,
    service: State<'_, Arc<AudioService>>,
    gate: State<'_, OperationGate>,
) -> Result<AudioSnapshot, String> {
    let _guard = gate.0.lock().await;
    Ok(service.stop_lane(lane).await)
}
