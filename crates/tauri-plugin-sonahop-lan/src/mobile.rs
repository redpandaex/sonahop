use crate::{AudioCapabilities, PermissionResponse};
use serde::de::DeserializeOwned;
use tauri::{
    AppHandle, Runtime,
    plugin::{PluginApi, PluginHandle},
};

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<SonahopLan<R>> {
    let handle =
        api.register_android_plugin("io.github.redpandaex.sonahop.lan", "LanPermissionPlugin")?;
    Ok(SonahopLan(handle))
}

pub struct SonahopLan<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> SonahopLan<R> {
    pub fn ensure_permission(&self) -> crate::Result<PermissionResponse> {
        self.0
            .run_mobile_plugin("ensurePermission", serde_json::json!({}))
            .map_err(Into::into)
    }
}

impl<R: Runtime> SonahopLan<R> {
    pub fn prepare_audio(&self, mode: &str) -> crate::Result<PermissionResponse> {
        self.0
            .run_mobile_plugin("prepareAudio", serde_json::json!({"mode":mode}))
            .map_err(Into::into)
    }
    pub fn start_playback(&self) -> crate::Result<()> {
        self.start_native("startPlayback")
    }
    pub fn start_capture(&self) -> crate::Result<()> {
        self.start_native("startCapture")
    }
    pub fn start_system_capture(&self) -> crate::Result<()> {
        self.start_native("startSystemCapture")
    }
    pub fn audio_capabilities(&self) -> crate::Result<AudioCapabilities> {
        self.0
            .run_mobile_plugin("audioCapabilities", serde_json::json!({}))
            .map_err(Into::into)
    }
    fn start_native(&self, command: &str) -> crate::Result<()> {
        let response: serde_json::Value =
            self.0.run_mobile_plugin(command, serde_json::json!({}))?;
        if response.get("started").and_then(|v| v.as_bool()) != Some(true) {
            return Err(crate::Error::Task("Android 音频未启动".into()));
        }
        Ok(())
    }
    pub fn stop_capture(&self) -> crate::Result<()> {
        self.stop_native("stopCapture")
    }
    pub fn stop_playback(&self) -> crate::Result<()> {
        self.stop_native("stopPlayback")
    }
    fn stop_native(&self, command: &str) -> crate::Result<()> {
        let _: serde_json::Value = self.0.run_mobile_plugin(command, serde_json::json!({}))?;
        Ok(())
    }
    pub fn stop_audio(&self) -> crate::Result<()> {
        let _: serde_json::Value = self
            .0
            .run_mobile_plugin("stopAudio", serde_json::json!({}))?;
        Ok(())
    }
}
