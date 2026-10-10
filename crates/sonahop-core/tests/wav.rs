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
    };
    assert!(WavRecorder::new(Cursor::new(Vec::new()), invalid, 2).is_err());
}
