use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
#[cfg(target_os = "windows")]
use tauri::Manager;
use tauri::State;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CloseBehavior {
    #[default]
    Tray,
    Exit,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Preferences {
    close_behavior: CloseBehavior,
}

pub struct DesktopSettings {
    preferences: Mutex<Preferences>,
    path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    close_behavior: CloseBehavior,
    tray_available: bool,
}

impl DesktopSettings {
    #[cfg(target_os = "windows")]
    pub fn load(app: &tauri::AppHandle) -> tauri::Result<Self> {
        let path = app.path().app_config_dir()?.join("desktop-settings.json");
        let preferences = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Preferences::default(),
        };
        Ok(Self {
            preferences: Mutex::new(preferences),
            path,
        })
    }

    #[cfg(target_os = "windows")]
    pub fn closes_to_tray(&self) -> bool {
        matches!(
            self.preferences
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .close_behavior,
            CloseBehavior::Tray
        )
    }

    fn snapshot(&self, app: &tauri::AppHandle) -> SettingsSnapshot {
        #[cfg(target_os = "windows")]
        let tray_available = app.tray_by_id(crate::tray::TRAY_ID).is_some();
        #[cfg(not(target_os = "windows"))]
        let tray_available = {
            let _ = app;
            false
        };
        SettingsSnapshot {
            close_behavior: self
                .preferences
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .close_behavior,
            tray_available,
        }
    }

    fn save(&self, close_behavior: CloseBehavior) -> Result<(), String> {
        let mut preferences = self.preferences.lock().unwrap_or_else(|e| e.into_inner());
        let next = Preferences { close_behavior };
        let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
        let directory = self.path.parent().ok_or("设置目录不可用")?;
        std::fs::create_dir_all(directory).map_err(|e| format!("创建设置目录失败：{e}"))?;
        let temporary = self.path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|e| format!("保存窗口设置失败：{e}"))?;
        std::fs::rename(&temporary, &self.path).map_err(|e| format!("更新窗口设置失败：{e}"))?;
        *preferences = next;
        Ok(())
    }
}

#[tauri::command]
pub fn get_desktop_settings(
    app: tauri::AppHandle,
    settings: State<'_, DesktopSettings>,
) -> SettingsSnapshot {
    settings.snapshot(&app)
}

#[tauri::command]
pub fn set_close_behavior(
    close_behavior: CloseBehavior,
    app: tauri::AppHandle,
    settings: State<'_, DesktopSettings>,
) -> Result<SettingsSnapshot, String> {
    settings.save(close_behavior)?;
    Ok(settings.snapshot(&app))
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopAction {
    StartSharing,
    StopSharing,
    ConnectDevices,
    Disconnect,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSessionSnapshot {
    pub revision: u64,
    pub action: Option<DesktopAction>,
    pub busy: bool,
    pub role: Option<sonahop_link::LinkRole>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn get_desktop_session(_app: tauri::AppHandle) -> DesktopSessionSnapshot {
    #[cfg(target_os = "windows")]
    if let Some(controller) = _app.try_state::<std::sync::Arc<crate::tray::TrayController>>() {
        return controller.snapshot();
    }
    DesktopSessionSnapshot::default()
}
