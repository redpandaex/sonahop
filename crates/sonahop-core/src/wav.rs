use crate::{AudioError, AudioFormat};
use serde::Serialize;
use std::io::{Seek, SeekFrom, Write};

const HEADER_BYTES: u64 = 80;
const FLOAT_SUBTYPE: [u8; 16] = [
    3, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71,
];

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
    writer: W,
    format: AudioFormat,
    max_frames: u64,
    frames_written: u64,
    peak_abs: f32,
}

impl<W: Write + Seek> WavRecorder<W> {
    pub fn new(mut writer: W, format: AudioFormat, max_frames: u64) -> Result<Self, AudioError> {
        let frame_bytes = format.bytes_per_frame()? as u64;
        if max_frames == 0 {
            return Err(AudioError::InvalidFormat(
                "recording frame budget must be positive",
            ));
        }
        max_frames
            .checked_mul(frame_bytes)
            .filter(|size| *size <= u64::from(u32::MAX) - HEADER_BYTES)
            .ok_or(AudioError::InvalidFormat(
                "recording exceeds RIFF file size limits",
            ))?;
        // WAVEFORMATEXTENSIBLE float32, with a fact chunk and an empty data chunk.
        // A zero mask leaves speaker positions unspecified rather than inventing them.
        let mut header = [0u8; HEADER_BYTES as usize];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&((HEADER_BYTES - 8) as u32).to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&40u32.to_le_bytes());
        header[20..22].copy_from_slice(&0xfffeu16.to_le_bytes());
        header[22..24].copy_from_slice(&format.channels.to_le_bytes());
        header[24..28].copy_from_slice(&format.sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&(format.sample_rate * frame_bytes as u32).to_le_bytes());
        header[32..34].copy_from_slice(&(frame_bytes as u16).to_le_bytes());
        header[34..36].copy_from_slice(&32u16.to_le_bytes());
        header[36..38].copy_from_slice(&22u16.to_le_bytes());
        header[38..40].copy_from_slice(&32u16.to_le_bytes());
        header[40..44].copy_from_slice(&format.channel_mask.unwrap_or(0).to_le_bytes());
        header[44..60].copy_from_slice(&FLOAT_SUBTYPE);
        header[60..64].copy_from_slice(b"fact");
        header[64..68].copy_from_slice(&4u32.to_le_bytes());
        header[72..76].copy_from_slice(b"data");
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&header)?;
        Ok(Self {
            writer,
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
            self.writer.write_all(&sample.to_le_bytes())?;
            self.peak_abs = self.peak_abs.max(sample.abs());
        }
        self.frames_written += frames as u64;
        Ok(frames)
    }

    pub fn is_full(&self) -> bool {
        self.frames_written == self.max_frames
    }

    pub fn finish(mut self) -> Result<RecordingSummary, AudioError> {
        let data_bytes = (self.frames_written * self.format.bytes_per_frame()? as u64) as u32;
        self.writer.seek(SeekFrom::Start(4))?;
        self.writer
            .write_all(&((HEADER_BYTES - 8) as u32 + data_bytes).to_le_bytes())?;
        self.writer.seek(SeekFrom::Start(68))?;
        self.writer
            .write_all(&(self.frames_written as u32).to_le_bytes())?;
        self.writer.seek(SeekFrom::Start(76))?;
        self.writer.write_all(&data_bytes.to_le_bytes())?;
        self.writer
            .seek(SeekFrom::Start(HEADER_BYTES + u64::from(data_bytes)))?;
        self.writer.flush()?;
        Ok(RecordingSummary {
            format: self.format,
            frames_written: self.frames_written,
            peak_abs: self.peak_abs,
        })
    }
}
