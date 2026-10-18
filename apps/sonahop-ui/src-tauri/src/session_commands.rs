use crate::audio::AudioService;
use sonahop_link::{LinkRole, LinkService, LinkSnapshot};
use std::sync::Arc;
use tauri::State;

/// User mutations share one gate; snapshots remain cheap and readable.
#[derive(Default)]
pub struct OperationGate(pub tokio::sync::Mutex<()>);

#[tauri::command]
pub async fn switch_connection_role(
    role: LinkRole,
    gate: State<'_, OperationGate>,
    link: State<'_, Arc<LinkService>>,
    audio: State<'_, Arc<AudioService>>,
) -> Result<LinkSnapshot, String> {
    let _guard = gate.0.lock().await;
    if link.snapshot().role == role && audio.snapshot().role == role {
        return Ok(link.snapshot());
    }
    link.begin_stop();
    let stopped = audio.stop().await;
    link.stop().await;
    if let Some(error) = stopped.last_error {
        return Err(error);
    }
    let snapshot = link.set_role(role).await?;
    audio.set_role(role).await?;
    Ok(snapshot)
}
