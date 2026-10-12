use crate::{AudioCapabilities, CaptureApps, PermissionResponse};
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
    pub fn prepare_audio(
        &self,
        _mode: &str,
        _package_name: Option<&str>,
    ) -> crate::Result<PermissionResponse> {
        Err(crate::Error::Task("音频权限请求仅用于 Android".into()))
    }
    pub fn list_capture_apps(&self) -> crate::Result<CaptureApps> {
        Err(crate::Error::Task("应用声音选择仅用于 Android".into()))
    }
    pub fn start_playback(&self) -> crate::Result<()> {
        Err(crate::Error::Task("Android 播放不适用于此平台".into()))
    }
    pub fn start_capture(&self) -> crate::Result<()> {
        Err(crate::Error::Task("Android 录音不适用于此平台".into()))
    }
    pub fn start_system_capture(&self) -> crate::Result<()> {
        Err(crate::Error::Task("Android 播放采集不适用于此平台".into()))
    }
    pub fn audio_capabilities(&self) -> crate::Result<AudioCapabilities> {
        Ok(AudioCapabilities {
            playback_capture: false,
            microphone_capture: false,
        })
    }
    pub fn stop_audio(&self) -> crate::Result<()> {
        Ok(())
    }
}
