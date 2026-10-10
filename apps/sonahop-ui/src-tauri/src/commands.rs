use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAppInfo {
    platform: &'static str,
    version: &'static str,
    ui_preview: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPlaybackDevice {
    id: String,
    name: String,
    is_default: bool,
}

#[tauri::command]
pub fn get_app_info() -> UiAppInfo {
    UiAppInfo {
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "android") {
            "android"
        } else {
            "other"
        },
        version: env!("CARGO_PKG_VERSION"),
        ui_preview: true,
    }
}

#[tauri::command]
pub async fn list_playback_devices() -> Result<Vec<UiPlaybackDevice>, String> {
    #[cfg(target_os = "windows")]
    {
        // WASAPI COM objects must be created and dropped on this MTA worker.
        // The WebView's UI thread may already own a different apartment.
        tauri::async_runtime::spawn_blocking(|| {
            sonahop_platform_windows::list_output_devices()
                .map(|devices| {
                    devices
                        .into_iter()
                        .map(|device| UiPlaybackDevice {
                            id: device.id,
                            name: device.name,
                            is_default: device.is_default,
                        })
                        .collect()
                })
                .map_err(|error| format!("获取播放设备失败：{error}"))
        })
        .await
        .map_err(|error| format!("设备读取任务失败：{error}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("当前平台不提供 Windows 播放设备枚举".into())
    }
}
