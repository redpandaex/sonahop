use crate::audio::AudioService;
use sonahop_link::{LinkService, LinkSnapshot};
use std::sync::Arc;
use tauri::State;

fn supported() -> Result<(), String> {
    if cfg!(target_os = "windows") || cfg!(target_os = "android") {
        Ok(())
    } else {
        Err("本阶段局域网连接仅接入 Windows 和 Android".into())
    }
}

#[tauri::command]
pub fn get_link_snapshot(service: State<'_, Arc<LinkService>>) -> Result<LinkSnapshot, String> {
    supported()?;
    Ok(service.snapshot())
}

#[tauri::command]
pub async fn start_link(
    name: String,
    service: State<'_, Arc<LinkService>>,
) -> Result<LinkSnapshot, String> {
    supported()?;
    let name = if name.trim().is_empty() {
        std::env::var("COMPUTERNAME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "SonaHop 设备".into())
    } else {
        name
    };
    service.start(name).await
}

#[tauri::command]
pub async fn stop_link(
    service: State<'_, Arc<LinkService>>,
    audio: State<'_, Arc<AudioService>>,
) -> Result<(), String> {
    audio.stop().await;
    service.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn connect_link(
    address: String,
    port: u16,
    pairing_code: String,
    service: State<'_, Arc<LinkService>>,
) -> Result<(), String> {
    supported()?;
    service.connect(address, port, pairing_code).await
}

#[tauri::command]
pub fn answer_link_request(
    request_id: String,
    approve: bool,
    service: State<'_, Arc<LinkService>>,
) -> Result<(), String> {
    supported()?;
    service.answer_request(&request_id, approve)
}

#[tauri::command]
pub fn disconnect_link_peer(
    peer_id: String,
    service: State<'_, Arc<LinkService>>,
) -> Result<(), String> {
    service.disconnect_peer(&peer_id)
}
