//! Fixed, bounded real-time audio framing. This crate has no OS or UI dependencies.
use opus::{Application, Bitrate, Channels, Decoder, Encoder};

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME_SAMPLES: usize = 960;
pub const FRAME_MILLIS: u64 = 20;
pub const MAX_OPUS_BYTES: usize = 1_275;
const HEADER_BYTES: usize = 26;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamKind {
    Listen,
    Microphone,
}
impl StreamKind {
    pub fn channels(self) -> usize {
        if self == Self::Listen { 2 } else { 1 }
    }
    fn opus_channels(self) -> Channels {
        if self == Self::Listen {
            Channels::Stereo
        } else {
            Channels::Mono
        }
    }
}

#[derive(Clone, Debug)]
pub struct MediaPacket {
    pub stream_id: [u8; 16],
    pub sequence: u64,
    pub kind: StreamKind,
    pub opus: Vec<u8>,
}
impl MediaPacket {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if !(HEADER_BYTES + 1..=HEADER_BYTES + MAX_OPUS_BYTES).contains(&bytes.len()) {
            return Err("音频包大小无效".into());
        }
        let kind = match (bytes[24], bytes[25]) {
            (1, 2) => StreamKind::Listen,
            (2, 1) => StreamKind::Microphone,
            _ => return Err("音频包格式不受支持".into()),
        };
        let mut stream_id = [0; 16];
        stream_id.copy_from_slice(&bytes[..16]);
        let mut sequence = [0; 8];
        sequence.copy_from_slice(&bytes[16..24]);
        Ok(Self {
            stream_id,
            sequence: u64::from_be_bytes(sequence),
            kind,
            opus: bytes[26..].to_vec(),
        })
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_BYTES + self.opus.len());
        bytes.extend_from_slice(&self.stream_id);
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.push(if self.kind == StreamKind::Listen {
            1
        } else {
            2
        });
        bytes.push(self.kind.channels() as u8);
        bytes.extend_from_slice(&self.opus);
        bytes
    }
}

pub struct AudioEncoder {
    codec: Encoder,
    kind: StreamKind,
    stream_id: [u8; 16],
    sequence: u64,
}
impl AudioEncoder {
    pub fn new(kind: StreamKind, stream_id: [u8; 16]) -> Result<Self, String> {
        let application = if kind == StreamKind::Listen {
            Application::Audio
        } else {
            Application::Voip
        };
        let mut codec = Encoder::new(SAMPLE_RATE, kind.opus_channels(), application)
            .map_err(|e| e.to_string())?;
        codec
            .set_bitrate(Bitrate::Bits(if kind == StreamKind::Listen {
                96_000
            } else {
                48_000
            }))
            .map_err(|e| e.to_string())?;
        codec.set_vbr(false).map_err(|e| e.to_string())?;
        codec.set_complexity(5).map_err(|e| e.to_string())?;
        Ok(Self {
            codec,
            kind,
            stream_id,
            sequence: 0,
        })
    }
    pub fn encode(&mut self, samples: &[f32]) -> Result<Vec<u8>, String> {
        if samples.len() != FRAME_SAMPLES * self.kind.channels()
            || samples.iter().any(|s| !s.is_finite())
        {
            return Err("编码需要完整的 20 ms 有限音频样本".into());
        }
        let mut output = vec![0; MAX_OPUS_BYTES];
        let count = self
            .codec
            .encode_float(samples, &mut output)
            .map_err(|e| e.to_string())?;
        output.truncate(count);
        let packet = MediaPacket {
            stream_id: self.stream_id,
            sequence: self.sequence,
            kind: self.kind,
            opus: output,
        };
        self.sequence = self.sequence.checked_add(1).ok_or("音频序号耗尽")?;
        Ok(packet.serialize())
    }
}

pub struct AudioDecoder {
    codec: Decoder,
    kind: StreamKind,
    stream_id: [u8; 16],
    last_sequence: Option<u64>,
}
impl AudioDecoder {
    pub fn new(kind: StreamKind, stream_id: [u8; 16]) -> Result<Self, String> {
        Ok(Self {
            codec: Decoder::new(SAMPLE_RATE, kind.opus_channels()).map_err(|e| e.to_string())?,
            kind,
            stream_id,
            last_sequence: None,
        })
    }
    /// Stale or reordered frames are discarded; a new stream requires a new decoder.
    pub fn decode(&mut self, packet: &MediaPacket) -> Result<Option<Vec<f32>>, String> {
        if packet.kind != self.kind || packet.stream_id != self.stream_id {
            return Err("音频会话不匹配".into());
        }
        if self
            .last_sequence
            .is_some_and(|last| packet.sequence <= last)
        {
            return Ok(None);
        }
        let mut pcm = vec![0.0; FRAME_SAMPLES * self.kind.channels()];
        let count = self
            .codec
            .decode_float(&packet.opus, &mut pcm, false)
            .map_err(|e| e.to_string())?;
        if count != FRAME_SAMPLES {
            return Err("音频帧必须为 20 ms".into());
        }
        if pcm.iter().any(|s| !s.is_finite()) {
            return Err("解码产生无效样本".into());
        }
        self.last_sequence = Some(packet.sequence);
        Ok(Some(pcm))
    }
    pub fn conceal(&mut self) -> Result<Vec<f32>, String> {
        let mut pcm = vec![0.0; FRAME_SAMPLES * self.kind.channels()];
        self.codec
            .decode_float(&[], &mut pcm, false)
            .map_err(|e| e.to_string())?;
        Ok(pcm)
    }
}

pub fn new_stream_id() -> [u8; 16] {
    *uuid::Uuid::new_v4().as_bytes()
}
pub fn stream_id_text(id: [u8; 16]) -> String {
    uuid::Uuid::from_bytes(id).to_string()
}
pub fn parse_stream_id(id: &str) -> Result<[u8; 16], String> {
    uuid::Uuid::parse_str(id)
        .map(|id| *id.as_bytes())
        .map_err(|_| "无效音频会话".into())
}
pub fn to_stereo(samples: &[f32], kind: StreamKind, gain: f32) -> Vec<f32> {
    let gain = gain.clamp(0.0, 1.0);
    if kind == StreamKind::Microphone {
        samples
            .iter()
            .flat_map(|s| {
                let value = (s * gain).clamp(-1.0, 1.0);
                [value, value]
            })
            .collect()
    } else {
        samples
            .iter()
            .map(|s| (s * gain).clamp(-1.0, 1.0))
            .collect()
    }
}
pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16)
        .collect()
}
