use crate::{PermissionResponse, Result, SonahopLanExt};
use tauri::{AppHandle, Runtime, command};
#[command]
pub(crate) async fn ensure_permission<R: Runtime>(app: AppHandle<R>) -> Result<PermissionResponse> {
    tauri::async_runtime::spawn_blocking(move || app.sonahop_lan().ensure_permission())
        .await
        .map_err(|e| crate::Error::Task(e.to_string()))?
}
#[command]
pub(crate) async fn prepare_audio<R: Runtime>(
    app: AppHandle<R>,
    mode: String,
) -> Result<PermissionResponse> {
    if mode != "listen" && mode != "microphone" {
        return Err(crate::Error::Task("无效音频模式".into()));
    }
    tauri::async_runtime::spawn_blocking(move || app.sonahop_lan().prepare_audio(&mode))
        .await
        .map_err(|e| crate::Error::Task(e.to_string()))?
}
