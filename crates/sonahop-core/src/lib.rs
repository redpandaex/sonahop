//! Portable audio formats and capture contracts.
use serde::Serialize;
use std::time::Duration;

pub const MAX_PACKET_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("invalid audio format: {0}")]
    InvalidFormat(&'static str),
    #[error("invalid audio packet: {0}")]
    InvalidPacket(&'static str),
    #[error("platform error: {0}")]
    Platform(String),
    #[error("no audio frames received during the capture window")]
    NoAudioData,
    #[error("this audio backend is available only on Windows")]
    UnsupportedPlatform,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    /// WAVE speaker-position bits, when known; no platform API types.
    pub channel_mask: Option<u32>,
}

impl AudioFormat {
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self, AudioError> {
        let format = Self {
            sample_rate,
            channels,
            channel_mask: None,
        };
        format.validate()?;
        Ok(format)
    }

    pub fn validate(self) -> Result<(), AudioError> {
        if self.sample_rate == 0 || self.channels == 0 {
            return Err(AudioError::InvalidFormat(
                "sample rate and channels must be positive",
            ));
        }
        let block_align = self
            .channels
            .checked_mul(4)
            .ok_or(AudioError::InvalidFormat(
                "float32 channel layout overflows block alignment",
            ))?;
        self.sample_rate
            .checked_mul(u32::from(block_align))
            .ok_or(AudioError::InvalidFormat(
                "audio byte rate exceeds WAV limits",
            ))?;
        Ok(())
    }

    pub fn bytes_per_frame(self) -> Result<usize, AudioError> {
        self.validate()?;
        Ok(usize::from(self.channels) * 4)
    }
}

#[derive(Debug)]
pub struct CapturePacket {
    pub samples: Vec<f32>,
    pub position: u64,
    pub discontinuity: bool,
}

pub trait AudioCapture {
    fn format(&self) -> AudioFormat;
    fn read_packet(&mut self, timeout: Duration) -> Result<Option<CapturePacket>, AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
}

/// Decode a complete interleaved float32 packet. Silent buffers are never read.
pub fn decode_f32_packet(
    format: AudioFormat,
    bytes: &[u8],
    frame_count: usize,
    silent: bool,
) -> Result<Vec<f32>, AudioError> {
    let bytes_per_frame = format.bytes_per_frame()?;
    let expected_bytes = frame_count
        .checked_mul(bytes_per_frame)
        .filter(|size| *size <= MAX_PACKET_BYTES)
        .ok_or(AudioError::InvalidPacket(
            "audio packet exceeds the bounded buffer size",
        ))?;
    let sample_count = expected_bytes / 4;
    if silent {
        return Ok(vec![0.0; sample_count]);
    }
    if bytes.len() != expected_bytes {
        return Err(AudioError::InvalidPacket(
            "buffer length does not match complete channel frames",
        ));
    }
    let mut samples = Vec::with_capacity(sample_count);
    for chunk in bytes.chunks_exact(4) {
        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if !sample.is_finite() {
            return Err(AudioError::InvalidPacket("non-finite audio sample"));
        }
        samples.push(sample);
    }
    Ok(samples)
}

mod wav;
pub use wav::{RecordingSummary, WavRecorder};
