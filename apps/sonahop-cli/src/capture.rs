use sonahop_core::{AudioCapture, AudioError, RecordingSummary};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, Write},
    path::Path,
    sync::atomic::AtomicBool,
    time::Duration,
};

pub struct CaptureReport {
    pub recording: RecordingSummary,
    pub elapsed_seconds: f64,
    pub packets: u64,
    pub discontinuities: u64,
    pub zero_sample_frames: u64,
    pub cancelled: bool,
}

pub fn reserve_output(path: &Path) -> Result<File, AudioError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                AudioError::Io(std::io::Error::new(
                    error.kind(),
                    format!("output already exists: {}", path.display()),
                ))
            } else {
                AudioError::Io(error)
            }
        })
}

pub fn capture_window<S: AudioCapture + ?Sized, W: Write + Seek>(
    source: &mut S,
    writer: W,
    duration: Duration,
    cancelled: &AtomicBool,
) -> Result<CaptureReport, AudioError> {
    use sonahop_core::WavRecorder;
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    let start = Instant::now();
    let format = source.format();
    let prepared = (|| {
        if duration.is_zero() {
            return Err(AudioError::InvalidFormat(
                "capture duration must be positive",
            ));
        }
        let deadline = start
            .checked_add(duration)
            .ok_or(AudioError::InvalidFormat(
                "capture deadline exceeds clock limits",
            ))?;
        let frames = duration
            .as_nanos()
            .checked_mul(u128::from(format.sample_rate))
            .map(|value| value.div_ceil(1_000_000_000))
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(AudioError::InvalidFormat(
                "capture frame budget exceeds limits",
            ))?;
        Ok((deadline, WavRecorder::new(writer, format, frames)?))
    })();
    let (deadline, mut recorder) = match prepared {
        Ok(value) => value,
        Err(error) => {
            return match source.stop() {
                Ok(()) => Err(error),
                Err(stop) => Err(AudioError::Platform(format!(
                    "{error}; stop also failed: {stop}"
                ))),
            };
        }
    };
    let mut packets = 0;
    let mut discontinuities = 0;
    let mut zero_sample_frames = 0;
    let work: Result<(), AudioError> = (|| {
        while !cancelled.load(Ordering::Relaxed) && !recorder.is_full() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let timeout = remaining.min(Duration::from_millis(50));
            if let Some(packet) = source.read_packet(timeout)? {
                let written = recorder.write_samples(&packet.samples)?;
                packets += 1;
                discontinuities += u64::from(packet.discontinuity);
                let channels = usize::from(format.channels);
                zero_sample_frames += packet.samples[..written * channels]
                    .chunks_exact(channels)
                    .filter(|frame| frame.iter().all(|sample| *sample == 0.0))
                    .count() as u64;
            }
        }
        Ok(())
    })();
    let was_cancelled = cancelled.load(Ordering::Relaxed);
    // Both cleanup operations run even when reading or writing failed.
    let stop = source.stop();
    let finished = recorder.finish();
    if let Err(error) = work {
        let mut details = Vec::new();
        if let Err(stop_error) = stop {
            details.push(format!("stop failed: {stop_error}"));
        }
        if let Err(wav_error) = finished {
            details.push(format!("WAV finalization failed: {wav_error}"));
        }
        return if details.is_empty() {
            Err(error)
        } else {
            Err(AudioError::Platform(format!(
                "{error}; {}",
                details.join("; ")
            )))
        };
    }
    stop?;
    let recording = finished?;
    if recording.frames_written == 0 && !was_cancelled {
        return Err(AudioError::NoAudioData);
    }
    Ok(CaptureReport {
        recording,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        packets,
        discontinuities,
        zero_sample_frames,
        cancelled: was_cancelled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sonahop_core::{AudioFormat, CapturePacket};
    use std::{collections::VecDeque, io::Cursor};

    struct Source {
        format: AudioFormat,
        packets: VecDeque<Result<Option<CapturePacket>, AudioError>>,
        stopped: bool,
        fail_stop: bool,
    }
    impl AudioCapture for Source {
        fn format(&self) -> AudioFormat {
            self.format
        }
        fn read_packet(&mut self, timeout: Duration) -> Result<Option<CapturePacket>, AudioError> {
            self.packets.pop_front().unwrap_or_else(|| {
                std::thread::sleep(timeout);
                Ok(None)
            })
        }
        fn stop(&mut self) -> Result<(), AudioError> {
            self.stopped = true;
            if self.fail_stop {
                Err(AudioError::Platform("stop failed".into()))
            } else {
                Ok(())
            }
        }
    }
    fn source() -> Source {
        Source {
            format: AudioFormat::new(8, 2).unwrap(),
            packets: VecDeque::from([Ok(Some(CapturePacket {
                samples: vec![0.25, -0.25, 0.5, -0.5, 1.0, -1.0],
                position: 0,
                discontinuity: false,
            }))]),
            stopped: false,
            fail_stop: false,
        }
    }

    #[test]
    fn trims_extra_frames_and_stops_the_source_before_returning() {
        let mut source = source();
        let mut bytes = Cursor::new(Vec::new());
        let report = capture_window(
            &mut source,
            &mut bytes,
            Duration::from_millis(250),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(source.stopped);
        assert_eq!(report.recording.frames_written, 2);
        let mut reader = hound_reader(bytes.into_inner());
        assert_eq!(reader.duration(), 2);
        assert_eq!(
            reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .unwrap(),
            [0.25, -0.25, 0.5, -0.5]
        );
    }

    fn hound_reader(bytes: Vec<u8>) -> hound::WavReader<Cursor<Vec<u8>>> {
        hound::WavReader::new(Cursor::new(bytes)).unwrap()
    }

    #[test]
    fn cancellation_stops_without_reading_real_audio() {
        let mut source = source();
        let mut bytes = Cursor::new(Vec::new());
        let report = capture_window(
            &mut source,
            &mut bytes,
            Duration::from_secs(1),
            &AtomicBool::new(true),
        )
        .unwrap();
        assert!(source.stopped);
        assert!(report.cancelled);
        assert_eq!(report.recording.frames_written, 0);
        assert_eq!(source.packets.len(), 1);
        assert_eq!(hound_reader(bytes.into_inner()).duration(), 0);
    }

    #[test]
    fn read_error_still_stops_and_finalizes_partial_wav() {
        let mut source = source();
        source
            .packets
            .push_back(Err(AudioError::Platform("device removed".into())));
        let mut bytes = Cursor::new(Vec::new());
        let error = capture_window(
            &mut source,
            &mut bytes,
            Duration::from_secs(1),
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert!(source.stopped);
        assert!(error.to_string().contains("device removed"));
        assert_eq!(hound_reader(bytes.into_inner()).duration(), 3);
    }

    #[test]
    fn no_data_reaches_its_deadline_and_closes_the_wav() {
        let mut source = source();
        source.packets.clear();
        let mut bytes = Cursor::new(Vec::new());
        let error = capture_window(
            &mut source,
            &mut bytes,
            Duration::from_millis(1),
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert!(source.stopped);
        assert!(matches!(error, AudioError::NoAudioData));
        assert_eq!(hound_reader(bytes.into_inner()).duration(), 0);
    }

    #[test]
    fn stop_error_is_reported_after_wav_is_finalized() {
        let mut source = source();
        source.fail_stop = true;
        let mut bytes = Cursor::new(Vec::new());
        let error = capture_window(
            &mut source,
            &mut bytes,
            Duration::from_millis(250),
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert!(source.stopped);
        assert!(error.to_string().contains("stop failed"));
        assert_eq!(hound_reader(bytes.into_inner()).duration(), 2);
    }
}
