use crate::PermissionResponse;
use serde::de::DeserializeOwned;
use tauri::{AppHandle, Runtime, plugin::PluginApi};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<SonahopLan<R>> {
    Ok(SonahopLan(app.clone()))
}

pub struct SonahopLan<R: Runtime>(AppHandle<R>);

impl<R: Runtime> SonahopLan<R> {
    pub fn ensure_permission(&self) -> crate::Result<PermissionResponse> {
        Ok(PermissionResponse {
            granted: true,
            device_name: self.0.package_info().name.clone(),
        })
    }
}

impl<R: Runtime> SonahopLan<R> {
    pub fn prepare_audio(&self, _mode: &str) -> crate::Result<PermissionResponse> {
        Err(crate::Error::Task("音频权限请求仅用于 Android".into()))
    }
    pub fn start_playback(&self) -> crate::Result<()> {
        Err(crate::Error::Task("Android 播放不适用于此平台".into()))
    }
    pub fn start_capture(&self) -> crate::Result<()> {
        Err(crate::Error::Task("Android 录音不适用于此平台".into()))
    }
    pub fn stop_audio(&self) -> crate::Result<()> {
        Ok(())
    }
}
