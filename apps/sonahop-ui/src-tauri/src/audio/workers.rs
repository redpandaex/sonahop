use super::backend::{Sink, Source};
use super::{AudioSource, Shared};
use sonahop_audio::{
    AudioDecoder, AudioEncoder, FRAME_MILLIS, FRAME_SAMPLES, MediaPacket, StreamKind, to_stereo,
};
use std::collections::VecDeque;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub(super) enum WorkerMode {
    Capture {
        source: AudioSource,
        device: Option<String>,
    },
    Playback {
        device: Option<String>,
    },
}
pub(super) struct Worker {
    cancel: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    pub media: SyncSender<MediaPacket>,
}
impl Worker {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn join(mut self) {
        self.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub(super) fn spawn(
    shared: Arc<Shared>,
    epoch: u64,
    mode: WorkerMode,
    kind: StreamKind,
    id: [u8; 16],
) -> Result<Worker, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = cancel.clone();
    let (media, receiver) = mpsc::sync_channel(3);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let join = thread::Builder::new()
        .name("sonahop-audio".into())
        .spawn(move || {
            let capture = matches!(mode, WorkerMode::Capture { .. });
            let result = if capture {
                capture_loop(&shared, epoch, mode, kind, id, &thread_cancel, ready_tx)
            } else {
                playback_loop(
                    &shared,
                    epoch,
                    mode,
                    kind,
                    id,
                    &thread_cancel,
                    receiver,
                    ready_tx,
                )
            };
            if let Err(error) = result {
                if !thread_cancel.load(Ordering::Acquire) {
                    shared.fail(kind, epoch, error);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    let worker = Worker {
        cancel,
        join: Some(join),
        media,
    };
    match ready_rx.recv_timeout(Duration::from_secs(15)) {
        Ok(Ok(())) => Ok(worker),
        Ok(Err(error)) => {
            worker.join();
            Err(error)
        }
        Err(error) => {
            worker.cancel();
            Err(format!("音频启动失败：{error}"))
        }
    }
}

fn capture_loop(
    shared: &Arc<Shared>,
    epoch: u64,
    mode: WorkerMode,
    kind: StreamKind,
    id: [u8; 16],
    cancel: &AtomicBool,
    ready: SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let initialized: Result<(Source, AudioEncoder), String> =
        (|| Ok((Source::new(shared, mode)?, AudioEncoder::new(kind, id)?)))();
    let (mut source, mut encoder) = match initialized {
        Ok(value) => {
            let _ = ready.send(Ok(()));
            value
        }
        Err(error) => {
            let _ = ready.send(Err(error.clone()));
            return Err(error);
        }
    };
    let frame_samples = FRAME_SAMPLES * kind.channels();
    let mut pending = VecDeque::with_capacity(frame_samples * 4);
    let mut next_frame = Instant::now() + Duration::from_millis(FRAME_MILLIS);
    while !cancel.load(Ordering::Acquire) {
        let samples = source.read(kind.channels())?.unwrap_or_default();
        if samples.len() > sonahop_core::MAX_PACKET_BYTES / 4
            || samples.iter().any(|s| !s.is_finite())
        {
            return Err("采集样本无效".into());
        }
        pending.extend(samples);
        while pending.len() > frame_samples * 4 {
            pending.drain(..frame_samples);
            shared.lock().lane_mut(kind).dropped_packets += 1;
        }
        while (pending.len() >= frame_samples || Instant::now() >= next_frame)
            && !cancel.load(Ordering::Acquire)
        {
            if pending.len() < frame_samples {
                pending.resize(frame_samples, 0.0);
            }
            next_frame += Duration::from_millis(FRAME_MILLIS);
            if next_frame + Duration::from_millis(40) < Instant::now() {
                next_frame = Instant::now() + Duration::from_millis(FRAME_MILLIS);
            }
            let (peers, gain, valid) = {
                let state = shared.lock();
                let peers = if kind == StreamKind::Listen {
                    state.listeners.keys().cloned().collect::<Vec<_>>()
                } else {
                    state.microphone.peer_id.iter().cloned().collect()
                };
                (
                    peers,
                    if kind == StreamKind::Microphone
                        || state.send_source == AudioSource::Microphone
                    {
                        state.gain as f32 / 100.0
                    } else {
                        1.0
                    },
                    state.lane(kind).epoch == epoch,
                )
            };
            if !valid {
                return Ok(());
            }
            let frame: Vec<f32> = pending
                .drain(..frame_samples)
                .map(|s| (s * gain).clamp(-1.0, 1.0))
                .collect();
            let peak = frame
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0_f32, f32::max)
                .clamp(0.0, 1.0);
            {
                let mut state = shared.lock();
                if state.lane(kind).epoch != epoch {
                    return Ok(());
                }
                let lane = state.lane_mut(kind);
                lane.peak_level = (lane.peak_level * 0.9).max(peak);
            }
            let packet = encoder.encode(&frame)?;
            for peer in peers {
                let delivered = shared.link.send_media(&peer, packet.clone()).is_ok();
                let mut state = shared.lock();
                if state.lane(kind).epoch != epoch {
                    return Ok(());
                }
                if delivered {
                    state.lane_mut(kind).sent_packets += 1;
                } else {
                    state.lane_mut(kind).dropped_packets += 1;
                }
            }
        }
    }
    Ok(())
}

fn playback_loop(
    shared: &Arc<Shared>,
    epoch: u64,
    mode: WorkerMode,
    kind: StreamKind,
    id: [u8; 16],
    cancel: &AtomicBool,
    incoming: Receiver<MediaPacket>,
    ready: SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let initialized: Result<(Sink, AudioDecoder), String> =
        (|| Ok((Sink::new(shared, mode)?, AudioDecoder::new(kind, id)?)))();
    let (mut sink, mut decoder) = match initialized {
        Ok(value) => {
            let _ = ready.send(Ok(()));
            value
        }
        Err(error) => {
            let _ = ready.send(Err(error.clone()));
            return Err(error);
        }
    };
    let frame_time = Duration::from_millis(FRAME_MILLIS);
    let mut next = Instant::now();
    let mut last_packet = Instant::now();
    let mut missed = 0;
    while !cancel.load(Ordering::Acquire) {
        let mut packet = match incoming.recv_timeout(
            next.saturating_duration_since(Instant::now())
                .max(Duration::from_millis(1)),
        ) {
            Ok(packet) => Some(packet),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };
        while let Ok(newer) = incoming.try_recv() {
            packet = Some(newer);
            shared.lock().lane_mut(kind).dropped_packets += 1;
        }
        if Instant::now() < next {
            thread::sleep(next.saturating_duration_since(Instant::now()));
        }
        let pcm = if let Some(packet) = packet {
            match decoder.decode(&packet) {
                Ok(Some(pcm)) => {
                    last_packet = Instant::now();
                    missed = 0;
                    pcm
                }
                Ok(None) => {
                    shared.lock().lane_mut(kind).dropped_packets += 1;
                    continue;
                }
                Err(error) => return Err(format!("音频解码失败：{error}")),
            }
        } else {
            missed += 1;
            if last_packet.elapsed() > Duration::from_secs(12) {
                return Err("超过 12 秒未收到音频，请重新开始".into());
            }
            if missed <= 3 {
                decoder.conceal()?
            } else {
                vec![0.0; FRAME_SAMPLES * kind.channels()]
            }
        };
        let peak = pcm
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max)
            .clamp(0.0, 1.0);
        let gain = {
            let mut state = shared.lock();
            if state.lane(kind).epoch != epoch {
                return Ok(());
            }
            let lane = state.lane_mut(kind);
            lane.peak_level = (lane.peak_level * 0.9).max(peak);
            1.0
        };
        if !sink.write(&to_stereo(&pcm, kind, gain))? {
            shared.lock().lane_mut(kind).dropped_packets += 1;
        }
        next += frame_time;
        if next + frame_time < Instant::now() {
            next = Instant::now();
        }
    }
    Ok(())
}
