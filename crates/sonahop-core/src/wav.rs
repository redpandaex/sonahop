use crate::{AudioError, AudioFormat};
use serde::Serialize;
use std::io::{Seek, Write};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RecordingSummary {
    pub format: AudioFormat,
    pub frames_written: u64,
    pub peak_abs: f32,
}

impl RecordingSummary {
    pub fn duration_seconds(self) -> f64 {
        self.frames_written as f64 / f64::from(self.format.sample_rate)
    }
}

pub struct WavRecorder<W: Write + Seek> {
    writer: hound::WavWriter<W>,
    format: AudioFormat,
    max_frames: u64,
    frames_written: u64,
    peak_abs: f32,
}

impl<W: Write + Seek> WavRecorder<W> {
    pub fn new(writer: W, format: AudioFormat, max_frames: u64) -> Result<Self, AudioError> {
        let frame_bytes = format.bytes_per_frame()? as u64;
        if max_frames == 0 {
            return Err(AudioError::InvalidFormat(
                "recording frame budget must be positive",
            ));
        }
        max_frames
            .checked_mul(frame_bytes)
            .filter(|size| *size <= u64::from(u32::MAX) - 128)
            .ok_or(AudioError::InvalidFormat(
                "recording exceeds RIFF file size limits",
            ))?;
        let spec = hound::WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        Ok(Self {
            writer: hound::WavWriter::new(writer, spec)?,
            format,
            max_frames,
            frames_written: 0,
            peak_abs: 0.0,
        })
    }

    /// Validate a complete packet before writing. Returns frames, not samples.
    pub fn write_samples(&mut self, samples: &[f32]) -> Result<usize, AudioError> {
        let channels = usize::from(self.format.channels);
        if samples.len() % channels != 0 {
            return Err(AudioError::InvalidPacket(
                "WAV input is not aligned to channel frames",
            ));
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(AudioError::InvalidPacket("non-finite WAV input"));
        }
        let incoming_frames = samples.len() / channels;
        let remaining = self.max_frames - self.frames_written;
        let frames = u64::try_from(incoming_frames)
            .map_err(|_| AudioError::InvalidPacket("frame count cannot be represented"))?
            .min(remaining);
        let frames = usize::try_from(frames)
            .map_err(|_| AudioError::InvalidPacket("frame count cannot be represented"))?;
        for &sample in &samples[..frames * channels] {
            self.writer.write_sample(sample)?;
            self.peak_abs = self.peak_abs.max(sample.abs());
        }
        self.frames_written += frames as u64;
        Ok(frames)
    }

    pub fn is_full(&self) -> bool {
        self.frames_written == self.max_frames
    }

    pub fn finish(self) -> Result<RecordingSummary, AudioError> {
        self.writer.finalize()?;
        Ok(RecordingSummary {
            format: self.format,
            frames_written: self.frames_written,
            peak_abs: self.peak_abs,
        })
    }
}
