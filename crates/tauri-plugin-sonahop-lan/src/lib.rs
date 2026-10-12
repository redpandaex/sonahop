use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

pub use models::*;

#[cfg(not(target_os = "android"))]
mod desktop;
#[cfg(target_os = "android")]
mod mobile;

#[cfg(target_os = "android")]
pub mod android_audio;
mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(not(target_os = "android"))]
use desktop::SonahopLan;
#[cfg(target_os = "android")]
use mobile::SonahopLan;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the sonahop-lan APIs.
pub trait SonahopLanExt<R: Runtime> {
    fn sonahop_lan(&self) -> &SonahopLan<R>;
}

impl<R: Runtime, T: Manager<R>> crate::SonahopLanExt<R> for T {
    fn sonahop_lan(&self) -> &SonahopLan<R> {
        self.state::<SonahopLan<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("sonahop-lan")
        .invoke_handler(tauri::generate_handler![
            commands::ensure_permission,
            commands::prepare_audio,
            commands::list_capture_apps
        ])
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let sonahop_lan = mobile::init(app, api)?;
            #[cfg(not(target_os = "android"))]
            let sonahop_lan = desktop::init(app, api)?;
            app.manage(sonahop_lan);
            Ok(())
        })
        .build()
}
