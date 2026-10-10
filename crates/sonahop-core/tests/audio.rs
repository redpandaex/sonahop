use sonahop_core::{AudioFormat, decode_f32_packet};

#[test]
fn decodes_interleaved_float_samples_without_changing_levels() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    let bytes = [0, 0, 0, 63, 0, 0, 0, 191];
    assert_eq!(
        decode_f32_packet(format, &bytes, 1, false).unwrap(),
        [0.5, -0.5]
    );
}

#[test]
fn silent_packets_ignore_their_unreadable_payload() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    assert_eq!(decode_f32_packet(format, &[], 2, true).unwrap(), [0.0; 4]);
}

#[test]
fn rejects_zero_rate_and_zero_channels() {
    assert!(AudioFormat::new(0, 2).is_err());
    assert!(AudioFormat::new(48_000, 0).is_err());
}

#[test]
fn rejects_invalid_format_even_when_constructed_directly() {
    let format = AudioFormat {
        sample_rate: 0,
        channels: 2,
    };
    assert!(decode_f32_packet(format, &[0; 8], 1, false).is_err());
}

#[test]
fn rejects_payload_with_an_incomplete_channel_frame() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    assert!(decode_f32_packet(format, &[0; 4], 1, false).is_err());
    assert!(decode_f32_packet(format, &[0; 7], 1, false).is_err());
    assert!(decode_f32_packet(format, &[0; 12], 1, false).is_err());
}

#[test]
fn rejects_non_finite_audio_before_it_can_reach_a_writer() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    let bytes = [0, 0, 192, 127, 0, 0, 0, 0];
    assert!(decode_f32_packet(format, &bytes, 1, false).is_err());
}

#[test]
fn rejects_oversized_silence_before_allocating_it() {
    let format = AudioFormat::new(48_000, 2).unwrap();
    assert!(decode_f32_packet(format, &[], usize::MAX, true).is_err());
}

#[test]
fn preserves_float_samples_above_integer_full_scale() {
    let format = AudioFormat::new(48_000, 1).unwrap();
    assert_eq!(
        decode_f32_packet(format, &[0, 0, 192, 63], 1, false).unwrap(),
        [1.5]
    );
}
