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
    package_name: Option<String>,
) -> Result<PermissionResponse> {
    if mode != "listen" && mode != "microphone" && mode != "playback" {
        return Err(crate::Error::Task("无效音频模式".into()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        app.sonahop_lan()
            .prepare_audio(&mode, package_name.as_deref())
    })
    .await
    .map_err(|e| crate::Error::Task(e.to_string()))?
}

#[command]
pub(crate) async fn list_capture_apps<R: Runtime>(app: AppHandle<R>) -> Result<crate::CaptureApps> {
    tauri::async_runtime::spawn_blocking(move || app.sonahop_lan().list_capture_apps())
        .await
        .map_err(|e| crate::Error::Task(e.to_string()))?
}
