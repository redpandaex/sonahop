use super::workers::WorkerMode;
use super::{AudioSource, Shared};
use sonahop_audio::FRAME_MILLIS;
#[cfg(target_os = "windows")]
use sonahop_core::AudioCapture;
use std::time::Duration;
#[cfg(target_os = "windows")]
use std::time::Instant;
#[cfg(target_os = "android")]
use tauri_plugin_sonahop_lan::SonahopLanExt;

/// Platform factories do not decide who hosts the connection.
pub(super) enum Source {
    #[cfg(target_os = "windows")]
    Windows(Box<dyn AudioCapture>),
    #[cfg(target_os = "android")]
    Android {
        app: tauri::AppHandle,
        channels: usize,
    },
    #[allow(dead_code)]
    Unsupported,
}
impl Source {
    pub(super) fn new(shared: &Shared, mode: WorkerMode) -> Result<Self, String> {
        let WorkerMode::Capture { source, device } = mode else {
            return Err("当前操作不是采集".into());
        };
        #[cfg(target_os = "windows")]
        {
            let _ = shared;
            let capture: Box<dyn AudioCapture> = match source {
                AudioSource::System => Box::new(
                    sonahop_platform_windows::SystemCapture::new().map_err(|e| e.to_string())?,
                ),
                AudioSource::Microphone => Box::new(
                    sonahop_platform_windows::MicrophoneCapture::new(device.as_deref())
                        .map_err(|e| e.to_string())?,
                ),
            };
            let format = capture.format();
            if format.sample_rate != 48000 || !matches!(format.channels, 1 | 2) {
                return Err("平台采集必须输出 48kHz 单声道或双声道".into());
            }
            Ok(Self::Windows(capture))
        }
        #[cfg(target_os = "android")]
        {
            let _ = device;
            tauri_plugin_sonahop_lan::android_audio::clear_capture();
            let channels = match source {
                AudioSource::System => {
                    shared
                        .app
                        .sonahop_lan()
                        .start_system_capture()
                        .map_err(|e| e.to_string())?;
                    2
                }
                AudioSource::Microphone => {
                    shared
                        .app
                        .sonahop_lan()
                        .start_capture()
                        .map_err(|e| e.to_string())?;
                    1
                }
            };
            Ok(Self::Android {
                app: shared.app.clone(),
                channels,
            })
        }
        #[cfg(not(any(target_os = "windows", target_os = "android")))]
        {
            let _ = (shared, source, device);
            Err("当前平台没有采集后端".into())
        }
    }
    pub(super) fn read(&mut self, target_channels: usize) -> Result<Option<Vec<f32>>, String> {
        let (samples, channels) = match self {
            #[cfg(target_os = "windows")]
            Self::Windows(capture) => (
                capture
                    .read_packet(Duration::from_millis(FRAME_MILLIS))
                    .map_err(|e| e.to_string())?
                    .map(|p| p.samples),
                capture.format().channels as usize,
            ),
            #[cfg(target_os = "android")]
            Self::Android { channels, .. } => {
                if let Some(error) = tauri_plugin_sonahop_lan::android_audio::take_error(true) {
                    return Err(error);
                }
                let samples = tauri_plugin_sonahop_lan::android_audio::captured_packet(
                    Duration::from_millis(FRAME_MILLIS),
                )?;
                (
                    samples.map(|values| {
                        values
                            .into_iter()
                            .map(|v| v as f32 / 32768.0)
                            .collect::<Vec<f32>>()
                    }),
                    *channels,
                )
            }
            Self::Unsupported => return Err("音频平台不可用".into()),
        };
        let Some(samples) = samples else {
            return Ok(None);
        };
        if channels == 0
            || samples.len() % channels != 0
            || samples.len() / channels * target_channels > sonahop_core::MAX_PACKET_BYTES / 4
        {
            return Err("采集声道或样本长度无效".into());
        }
        let samples = match (channels, target_channels) {
            (a, b) if a == b => samples,
            (1, 2) => samples.into_iter().flat_map(|v| [v, v]).collect(),
            (2, 1) => samples
                .chunks_exact(2)
                .map(|v| (v[0] + v[1]) * 0.5)
                .collect(),
            _ => return Err("采集声道转换不受支持".into()),
        };
        Ok(Some(samples))
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(capture) => {
                let _ = capture.stop();
            }
            #[cfg(target_os = "android")]
            Self::Android { app, .. } => {
                let _ = app.sonahop_lan().stop_capture();
            }
            _ => {}
        }
    }
}
pub(super) enum Sink {
    #[cfg(target_os = "windows")]
    Windows(sonahop_platform_windows::AudioOutput),
    #[cfg(target_os = "android")]
    Android(tauri::AppHandle),
    #[allow(dead_code)]
    Unsupported,
}
impl Sink {
    pub(super) fn new(shared: &Shared, mode: WorkerMode) -> Result<Self, String> {
        let WorkerMode::Playback { device } = mode else {
            return Err("当前操作不是播放".into());
        };
        #[cfg(target_os = "windows")]
        {
            let _ = shared;
            let device = match device {
                Some(device) if !device.trim().is_empty() => device,
                _ => sonahop_platform_windows::list_output_devices()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .find(|d| d.is_default)
                    .map(|d| d.id)
                    .ok_or("没有默认播放设备，请选择播放出口")?,
            };
            sonahop_platform_windows::AudioOutput::new(&device)
                .map(Self::Windows)
                .map_err(|e| e.to_string())
        }
        #[cfg(target_os = "android")]
        {
            let _ = device;
            shared
                .app
                .sonahop_lan()
                .start_playback()
                .map_err(|e| e.to_string())?;
            Ok(Self::Android(shared.app.clone()))
        }
        #[cfg(not(any(target_os = "windows", target_os = "android")))]
        {
            let _ = (shared, device);
            Err("当前平台没有播放后端".into())
        }
    }
    pub(super) fn write(&mut self, pcm: &[f32]) -> Result<bool, String> {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(output) => {
                let deadline = Instant::now() + Duration::from_millis(30);
                let mut consumed = 0;
                while consumed < pcm.len() && Instant::now() < deadline {
                    consumed += output
                        .write(&pcm[consumed..], Duration::from_millis(5))
                        .map_err(|e| e.to_string())?;
                }
                Ok(consumed == pcm.len())
            }
            #[cfg(target_os = "android")]
            Self::Android(_) => {
                if let Some(error) = tauri_plugin_sonahop_lan::android_audio::take_error(false) {
                    return Err(error);
                }
                tauri_plugin_sonahop_lan::android_audio::write_playback(&sonahop_audio::to_i16(
                    pcm,
                ))?;
                Ok(true)
            }
            Self::Unsupported => Err("音频平台不可用".into()),
        }
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        match self {
            #[cfg(target_os = "windows")]
            Self::Windows(output) => {
                let _ = output.stop();
            }
            #[cfg(target_os = "android")]
            Self::Android(app) => {
                let _ = app.sonahop_lan().stop_playback();
            }
            _ => {}
        }
    }
}
