//! Windows playback-device loopback capture.
#[cfg(not(windows))]
use sonahop_core::{AudioCapture, AudioError};

#[derive(Debug, Clone)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::{list_output_devices, open_loopback};

#[cfg(not(windows))]
pub fn list_output_devices() -> Result<Vec<OutputDevice>, AudioError> {
    Err(AudioError::UnsupportedPlatform)
}
#[cfg(not(windows))]
pub fn open_loopback(_device_id: Option<&str>) -> Result<Box<dyn AudioCapture>, AudioError> {
    Err(AudioError::UnsupportedPlatform)
}
