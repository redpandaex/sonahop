//! Native Android PCM bridge. No audio packets cross Tauri's JSON command channel.
//!
//! Java serializes callbacks by capture/playback session before entering JNI. The
//! queue holds three 20 ms mono or stereo packets; overflow discards the oldest packet so a
//! delayed network sender cannot accumulate stale microphone audio.
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JByteArray, JClass, JString, JValue},
};
use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex, OnceLock},
    time::Duration,
};

const CAPTURE_SAMPLES: usize = 960;
const CAPTURE_QUEUE_PACKETS: usize = 3;
const MAX_PLAYBACK_SAMPLES: usize = 3840;

struct JavaBridge {
    vm: JavaVM,
    owner: GlobalRef,
}

static JAVA: OnceLock<JavaBridge> = OnceLock::new();
static CAPTURE: OnceLock<(Mutex<AudioState>, Condvar)> = OnceLock::new();
static PLAYBACK: OnceLock<(Mutex<AudioState>, Condvar)> = OnceLock::new();
static NOTIFICATION_STOP: OnceLock<Box<dyn Fn() -> bool + Send + Sync>> = OnceLock::new();

/// The application coordinator cancels both active and waiting control lanes.
pub fn set_notification_stop_handler(handler: impl Fn() -> bool + Send + Sync + 'static) {
    let _ = NOTIFICATION_STOP.set(Box::new(handler));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_redpandaex_sonahop_lan_AudioBridge_nativeStopRequested(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jni::sys::jboolean {
    if NOTIFICATION_STOP.get().is_some_and(|handler| handler()) {
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

struct AudioState {
    packets: VecDeque<Vec<i16>>,
    error: Option<String>,
    stopped: bool,
    generation: u64,
    java_session: i64,
    capture_channels: usize,
}

fn audio(capture: bool) -> &'static (Mutex<AudioState>, Condvar) {
    let slot = if capture { &CAPTURE } else { &PLAYBACK };
    slot.get_or_init(|| {
        (
            Mutex::new(AudioState {
                packets: VecDeque::with_capacity(CAPTURE_QUEUE_PACKETS),
                error: None,
                stopped: true,
                generation: 0,
                java_session: 0,
                capture_channels: 0,
            }),
            Condvar::new(),
        )
    })
}

/// Receive one 20 ms, 48 kHz mono microphone or stereo playback PCM16 packet.
/// Hardware/service failures wake the waiter immediately.
pub fn captured_packet(timeout: Duration) -> Result<Option<Vec<i16>>, String> {
    let (mutex, wake) = audio(true);
    let state = mutex
        .lock()
        .map_err(|_| "Android capture state lock failed")?;
    let generation = state.generation;
    let (mut state, _) = wake
        .wait_timeout_while(state, timeout, |state| {
            state.packets.is_empty()
                && !state.stopped
                && state.error.is_none()
                && state.generation == generation
        })
        .map_err(|_| "Android capture state wait failed")?;
    if state.generation != generation {
        return Err("Android capture session changed".into());
    }
    if let Some(error) = &state.error {
        return Err(error.clone());
    }
    if state.stopped {
        return Err("Android native audio is stopped".into());
    }
    Ok(state.packets.pop_front())
}

pub fn clear_capture() {
    if let Ok(mut state) = audio(true).0.lock() {
        state.packets.clear();
    }
}

/// Called by the Rust playback worker, never by the UI or a Tauri callback.
/// The native track has a 20 ms write deadline and no unbounded Java queue.
pub fn write_playback(samples: &[i16]) -> Result<(), String> {
    if samples.is_empty() || samples.len() % 2 != 0 || samples.len() > MAX_PLAYBACK_SAMPLES {
        return Err("Android playback needs one bounded stereo PCM16 packet".into());
    }
    let bridge = JAVA.get().ok_or("Android audio JNI is not registered")?;
    let (generation, java_session) = {
        let state = audio(false)
            .0
            .lock()
            .map_err(|_| "Android audio state lock failed")?;
        if state.stopped {
            return Err(state
                .error
                .clone()
                .unwrap_or_else(|| "Android native audio is stopped".into()));
        }
        (state.generation, state.java_session)
    };
    let bytes: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    // JNI attaches this worker once and detaches when the Rust thread exits.
    let mut env = bridge
        .vm
        .attach_current_thread_permanently()
        .map_err(|error| format!("Cannot attach Android playback worker to JVM: {error}"))?;
    // Attached native threads do not have implicit per-call local JNI frames.
    let written = env.with_local_frame(8, |env| -> jni::errors::Result<i32> {
        let pcm = env.byte_array_from_slice(&bytes)?;
        env.call_static_method(
            &bridge.owner,
            "writePlayback",
            "([BJ)I",
            &[JValue::Object(pcm.as_ref()), JValue::Long(java_session)],
        )?
        .i()
    });
    let written = match written {
        Ok(written) => written,
        Err(error) => {
            let _ = env.exception_clear();
            return Err(format!("Android AudioTrack JNI write failed: {error}"));
        }
    };
    let state = audio(false)
        .0
        .lock()
        .map_err(|_| "Android audio state lock failed")?;
    if state.generation != generation || state.stopped {
        return Err(state
            .error
            .clone()
            .unwrap_or_else(|| "Android playback session stopped or changed".into()));
    }
    if written != bytes.len() as i32 {
        return Err(format!(
            "Android AudioTrack accepted {written} of {} PCM bytes",
            bytes.len()
        ));
    }
    Ok(())
}

/// Drained by the coordinator so notification stop/focus loss update app state.
pub fn take_error(capture: bool) -> Option<String> {
    audio(capture).0.lock().ok()?.error.take()
}

fn failure(message: String) {
    for capture in [true, false] {
        let (mutex, wake) = audio(capture);
        if let Ok(mut state) = mutex.lock() {
            state.error = Some(message.chars().take(512).collect());
            state.stopped = true;
            state.packets.clear();
            wake.notify_all();
        }
    }
}

// These exports are in sonahop_ui_lib, already loaded by Tauri before commands.
// AudioBridge is preserved by consumer-rules.pro; the cached class reference
// avoids FindClass failures on a Rust-created JVM thread.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_redpandaex_sonahop_lan_AudioBridge_nativeRegister(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    owner: JClass<'_>,
) {
    if JAVA.get().is_some() {
        return;
    }
    let registered = (|| -> jni::errors::Result<JavaBridge> {
        Ok(JavaBridge {
            vm: env.get_java_vm()?,
            owner: env.new_global_ref(owner)?,
        })
    })();
    match registered {
        Ok(bridge) => {
            let _ = JAVA.set(bridge);
        }
        Err(error) => {
            let message = format!("Cannot register SonaHop native audio: {error}");
            failure(message.clone());
            let _ = env.exception_clear();
            let _ = env.throw_new("java/lang/IllegalStateException", message);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_redpandaex_sonahop_lan_AudioBridge_nativeStarted(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    session_id: jni::sys::jlong,
    capture_channels: jni::sys::jint,
) {
    if session_id <= 0 || !(0..=2).contains(&capture_channels) {
        let _ = env.throw_new(
            "java/lang/IllegalArgumentException",
            "Invalid audio session or capture channels",
        );
        return;
    }
    let (mutex, wake) = audio(capture_channels > 0);
    if let Ok(mut state) = mutex.lock() {
        state.generation = state.generation.wrapping_add(1);
        state.packets.clear();
        state.error = None;
        state.java_session = session_id;
        state.capture_channels = capture_channels as usize;
        state.stopped = false;
        wake.notify_all();
    }
}
fn failure_for_session(session_id: i64, message: String) {
    for capture in [true, false] {
        let (mutex, wake) = audio(capture);
        if let Ok(mut state) = mutex.lock() {
            if state.java_session != session_id {
                continue;
            }
            if state.error.is_none() {
                state.error = Some(message.chars().take(512).collect());
            }
            state.stopped = true;
            state.packets.clear();
            wake.notify_all();
        }
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_redpandaex_sonahop_lan_AudioBridge_nativeCaptured(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
    pcm: JByteArray<'_>,
    session_id: jni::sys::jlong,
) {
    let expected = {
        let Ok(state) = audio(true).0.lock() else {
            return;
        };
        if state.stopped || state.java_session != session_id {
            return;
        }
        CAPTURE_SAMPLES * state.capture_channels * 2
    };
    if expected == 0 || env.get_array_length(&pcm).ok() != Some(expected as i32) {
        failure_for_session(
            session_id,
            "Android capture delivered an invalid PCM packet".into(),
        );
        return;
    }
    let bytes = match env.convert_byte_array(&pcm) {
        Ok(bytes) => bytes,
        Err(error) => {
            failure_for_session(
                session_id,
                format!("Cannot copy Android capture packet: {error}"),
            );
            return;
        }
    };
    let packet = bytes
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let (mutex, wake) = audio(true);
    if let Ok(mut state) = mutex.lock() {
        if state.stopped || state.java_session != session_id {
            return;
        }
        if state.packets.len() == CAPTURE_QUEUE_PACKETS {
            state.packets.pop_front();
        }
        state.packets.push_back(packet);
        wake.notify_one();
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_redpandaex_sonahop_lan_AudioBridge_nativeAudioFailure(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    message: JString<'_>,
    session_id: jni::sys::jlong,
) {
    let message = env
        .get_string(&message)
        .map(|value| value.into())
        .unwrap_or_else(|_| "Android native audio failed".into());
    failure_for_session(session_id, message);
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_redpandaex_sonahop_lan_AudioBridge_nativeStopped(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    session_id: jni::sys::jlong,
) {
    for capture in [true, false] {
        let (mutex, wake) = audio(capture);
        if let Ok(mut state) = mutex.lock() {
            if state.java_session != session_id {
                continue;
            }
            state.generation = state.generation.wrapping_add(1);
            state.stopped = true;
            state.capture_channels = 0;
            state.packets.clear();
            wake.notify_all();
        }
    }
}
