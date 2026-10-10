use sonahop_core::{AudioFormat, WavRecorder};
use std::io::Cursor;

#[test]
fn wav_preserves_format_samples_and_a_complete_frame_limit() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = WavRecorder::new(&mut bytes, format, 2).unwrap();
    assert_eq!(
        writer
            .write_samples(&[0.25, -0.25, 0.5, -0.5, 1.0, -1.0])
            .unwrap(),
        2
    );
    assert_eq!(writer.write_samples(&[0.75, -0.75]).unwrap(), 0);
    let summary = writer.finish().unwrap();
    assert_eq!(summary.frames_written, 2);
    let mut reader = hound::WavReader::new(Cursor::new(bytes.into_inner())).unwrap();
    assert_eq!(reader.spec().sample_rate, 48_000);
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().bits_per_sample, 32);
    assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
    assert_eq!(reader.duration(), 2);
    let samples: Vec<_> = reader.samples::<f32>().collect::<Result<_, _>>().unwrap();
    assert_eq!(samples, [0.25, -0.25, 0.5, -0.5]);
}

#[test]
fn malformed_packet_does_not_partially_change_the_wav() {
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = WavRecorder::new(&mut bytes, AudioFormat::new(44_100, 2).unwrap(), 8).unwrap();
    assert!(writer.write_samples(&[0.5]).is_err());
    assert!(writer.write_samples(&[0.25, f32::NAN]).is_err());
    assert_eq!(writer.write_samples(&[0.5, -0.5]).unwrap(), 1);
    assert_eq!(writer.finish().unwrap().frames_written, 1);
    let mut reader = hound::WavReader::new(Cursor::new(bytes.into_inner())).unwrap();
    assert_eq!(reader.duration(), 1);
    assert_eq!(
        reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap(),
        [0.5, -0.5]
    );
}

#[test]
fn wav_rejects_limits_that_cannot_fit_a_riff_file() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    assert!(WavRecorder::new(Cursor::new(Vec::new()), format, u64::MAX).is_err());
}

#[test]
fn wav_rejects_zero_budget_and_invalid_direct_format() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    assert!(WavRecorder::new(Cursor::new(Vec::new()), format, 0).is_err());
    let invalid = AudioFormat {
        sample_rate: 0,
        channels: 2,
        channel_mask: None,
    };
    assert!(WavRecorder::new(Cursor::new(Vec::new()), invalid, 2).is_err());
}

fn wav_channel_mask(bytes: &[u8]) -> u32 {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let payload = &bytes[offset + 8..offset + 8 + size];
        if &bytes[offset..offset + 4] == b"fmt " {
            assert_eq!(
                u16::from_le_bytes(payload[0..2].try_into().unwrap()),
                0xfffe
            );
            return u32::from_le_bytes(payload[20..24].try_into().unwrap());
        }
        offset += 8 + size + (size % 2);
    }
    panic!("missing fmt chunk");
}

#[test]
fn wav_does_not_invent_speaker_positions_for_unknown_layout() {
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = WavRecorder::new(&mut bytes, AudioFormat::new(48_000, 6).unwrap(), 1).unwrap();
    writer.write_samples(&[0.0; 6]).unwrap();
    writer.finish().unwrap();
    assert_eq!(wav_channel_mask(bytes.get_ref()), 0);
}

#[test]
fn wav_preserves_side_surround_layout_and_channel_samples() {
    for (channels, mask) in [(6u16, 0x60fu32), (8, 0x63f)] {
        let mut format = AudioFormat::new(44_100, channels).unwrap();
        format.channel_mask = Some(mask);
        let mut bytes = Cursor::new(Vec::new());
        let mut writer = WavRecorder::new(&mut bytes, format, 1).unwrap();
        let samples: Vec<_> = (0..channels)
            .map(|channel| f32::from(channel) * 0.125)
            .collect();
        writer.write_samples(&samples).unwrap();
        assert_eq!(writer.finish().unwrap().format.channel_mask, Some(mask));
        assert_eq!(wav_channel_mask(bytes.get_ref()), mask);
        assert_eq!(
            u32::from_le_bytes(bytes.get_ref()[4..8].try_into().unwrap()) as usize,
            bytes.get_ref().len() - 8
        );
        let mut reader = hound::WavReader::new(Cursor::new(bytes.into_inner())).unwrap();
        assert_eq!(reader.spec().channels, channels);
        assert_eq!(reader.duration(), 1);
        assert_eq!(
            reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            samples
        );
    }
}
