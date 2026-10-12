use serde::Serialize;
#[cfg(target_os = "android")]
use tauri_plugin_sonahop_lan::SonahopLanExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAppInfo {
    platform: &'static str,
    version: &'static str,
    ui_preview: bool,
    system_capture_supported: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPlaybackDevice {
    id: String,
    name: String,
    is_default: bool,
}

#[tauri::command]
pub async fn get_app_info(_app: tauri::AppHandle) -> UiAppInfo {
    #[cfg(target_os = "android")]
    let system_capture_supported = tauri::async_runtime::spawn_blocking(move || {
        _app.sonahop_lan()
            .audio_capabilities()
            .map(|c| c.playback_capture)
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false);
    #[cfg(not(target_os = "android"))]
    let system_capture_supported = cfg!(target_os = "windows");
    UiAppInfo {
        platform: if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "android") {
            "android"
        } else {
            "other"
        },
        version: env!("CARGO_PKG_VERSION"),
        ui_preview: false,
        system_capture_supported,
    }
}
#[tauri::command]
pub async fn list_playback_devices() -> Result<Vec<UiPlaybackDevice>, String> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(|| {
            sonahop_platform_windows::list_output_devices()
                .map(|devices| {
                    devices
                        .into_iter()
                        .map(|d| UiPlaybackDevice {
                            id: d.id,
                            name: d.name,
                            is_default: d.is_default,
                        })
                        .collect()
                })
                .map_err(|e| format!("获取播放设备失败：{e}"))
        })
        .await
        .map_err(|e| format!("设备读取任务失败：{e}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("当前平台不提供 Windows 播放设备枚举".into())
    }
}
#[tauri::command]
pub async fn list_capture_devices() -> Result<Vec<UiPlaybackDevice>, String> {
    #[cfg(target_os = "windows")]
    {
        tauri::async_runtime::spawn_blocking(|| {
            sonahop_platform_windows::list_input_devices()
                .map(|devices| {
                    devices
                        .into_iter()
                        .map(|d| UiPlaybackDevice {
                            id: d.id,
                            name: d.name,
                            is_default: d.is_default,
                        })
                        .collect()
                })
                .map_err(|e| format!("获取录音设备失败：{e}"))
        })
        .await
        .map_err(|e| format!("录音设备读取任务失败：{e}"))?
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(Vec::new())
    }
}
