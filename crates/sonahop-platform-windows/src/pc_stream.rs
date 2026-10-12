//! Windows worker-thread audio I/O. No endpoint routing or volume changes.
use ::windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK,
            AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, AUDIOCLIENT_ACTIVATION_PARAMS,
            AUDIOCLIENT_ACTIVATION_PARAMS_0, AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS, ActivateAudioInterfaceAsync,
            IActivateAudioInterfaceAsyncOperation, IActivateAudioInterfaceCompletionHandler,
            IActivateAudioInterfaceCompletionHandler_Impl, IAudioCaptureClient, IAudioClient,
            IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE,
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
        },
        System::{
            Com::{
                BLOB, CLSCTX_ALL, CoCreateInstance,
                StructuredStorage::{
                    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
                },
            },
            Threading::{CreateEventW, WaitForSingleObject},
            Variant::VT_BLOB,
        },
    },
    core::{HRESULT, HSTRING, IUnknown, Interface, PCWSTR, Ref, implement},
};
use sonahop_core::{
    AudioCapture, AudioError, AudioFormat, CapturePacket, MAX_PACKET_BYTES, decode_f32_packet,
};
use std::{
    marker::PhantomData,
    mem::{ManuallyDrop, size_of},
    rc::Rc,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
use wasapi::{DeviceEnumerator, Direction, SampleType, WaveFormat};

const CHANNELS: usize = 2;
const BYTES_PER_FRAME: usize = CHANNELS * size_of::<f32>();
const FORMAT: AudioFormat = AudioFormat {
    sample_rate: 48_000,
    channels: CHANNELS as u16,
    channel_mask: Some(3),
};
const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(10);
const STREAM_BUFFER_HNS: i64 = 200_000; // Requested 20 ms; the engine may round up.

fn platform(error: impl std::fmt::Display) -> AudioError {
    AudioError::Platform(error.to_string())
}

fn wave_format() -> WaveFormat {
    WaveFormat::new(32, 32, &SampleType::Float, 48_000, CHANNELS, Some(3))
}

/// The Rc marker keeps initialization, interfaces and teardown on one thread.
struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    fn new() -> Result<Self, AudioError> {
        wasapi::initialize_mta().ok().map_err(|error| {
            platform(format!(
                "audio I/O requires a dedicated MTA worker thread: {error}"
            ))
        })?;
        Ok(Self(PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}

struct Event(HANDLE);
impl Event {
    fn new() -> Result<Self, AudioError> {
        // SAFETY: this creates an unnamed auto-reset event owned by this wrapper.
        unsafe { CreateEventW(None, false, false, PCWSTR::null()) }
            .map(Self)
            .map_err(platform)
    }

    fn wait(&self, remaining: Duration) -> Result<bool, AudioError> {
        // Round up sub-millisecond waits, and never use the INFINITE sentinel.
        let millis = remaining
            .as_millis()
            .saturating_add(u128::from(remaining.subsec_nanos() % 1_000_000 != 0))
            .min(u128::from(u32::MAX - 1)) as u32;
        // SAFETY: the stream owns a live event throughout this wait.
        match unsafe { WaitForSingleObject(self.0, millis) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(platform(::windows::core::Error::from_thread())),
        }
    }
}
impl Drop for Event {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the event handle.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Both the PROPVARIANT and its blob remain live until activation completes,
/// including when the caller times out and Windows later invokes the callback.
struct ActivationStorage {
    _params: Box<AUDIOCLIENT_ACTIVATION_PARAMS>,
    // PROPVARIANT automatically calls PropVariantClear on drop. Its BLOB here
    // borrows _params, so only the Rust Box may release that allocation.
    variant: ManuallyDrop<PROPVARIANT>,
}
impl ActivationStorage {
    fn new() -> Self {
        let mut params = Box::new(AUDIOCLIENT_ACTIVATION_PARAMS {
            ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                    TargetProcessId: std::process::id(),
                    ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE,
                },
            },
        });
        let variant = PROPVARIANT {
            Anonymous: PROPVARIANT_0 {
                Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                    vt: VT_BLOB,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: PROPVARIANT_0_0_0 {
                        blob: BLOB {
                            cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
                            pBlobData: std::ptr::from_mut(params.as_mut()).cast(),
                        },
                    },
                }),
            },
        };
        // Do not PropVariantClear: the blob belongs to the Rust Box, not COM.
        Self {
            _params: params,
            variant: ManuallyDrop::new(variant),
        }
    }
}
// SAFETY: this private storage is immutable after construction, its sole raw
// pointer refers to its own stable Box, and it contains no COM interfaces.
unsafe impl Send for ActivationStorage {}
unsafe impl Sync for ActivationStorage {}

// windows-rs implements an agile handler/free-threaded marshaler by default.
#[implement(IActivateAudioInterfaceCompletionHandler)]
struct ActivationHandler {
    _storage: Arc<ActivationStorage>,
    completed: Arc<(Mutex<bool>, Condvar)>,
}
impl IActivateAudioInterfaceCompletionHandler_Impl for ActivationHandler_Impl {
    fn ActivateCompleted(
        &self,
        _operation: Ref<IActivateAudioInterfaceAsyncOperation>,
    ) -> ::windows::core::Result<()> {
        let (lock, signal) = &*self.completed;
        // Never unwind through the COM callback, even after mutex poisoning.
        let mut completed = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *completed = true;
        signal.notify_one();
        Ok(())
    }
}

fn activate_process_loopback() -> Result<IAudioClient, AudioError> {
    let storage = Arc::new(ActivationStorage::new());
    let completed = Arc::new((Mutex::new(false), Condvar::new()));
    let callback: IActivateAudioInterfaceCompletionHandler = ActivationHandler {
        _storage: storage.clone(),
        completed: completed.clone(),
    }
    .into();
    // SAFETY: activation parameters and their blob are retained by the handler;
    // Windows retains that handler until the asynchronous callback completes.
    let operation = unsafe {
        ActivateAudioInterfaceAsync(
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &IAudioClient::IID,
            Some(std::ptr::from_ref(&*storage.variant)),
            &callback,
        )
    }
    .map_err(|error| {
        platform(format!(
            "process loopback activation failed (requires Windows build 20348 or newer): {error}"
        ))
    })?;
    let (lock, signal) = &*completed;
    let ready = lock.lock().map_err(platform)?;
    let (ready, _) = signal
        .wait_timeout_while(ready, ACTIVATION_TIMEOUT, |ready| !*ready)
        .map_err(platform)?;
    if !*ready {
        return Err(platform(
            "process loopback activation timed out after 10 seconds",
        ));
    }
    drop(ready);
    let mut result = HRESULT(0);
    let mut unknown: Option<IUnknown> = None;
    // SAFETY: activation completed; output pointers remain valid for the call.
    unsafe { operation.GetActivateResult(&mut result, &mut unknown) }.map_err(platform)?;
    result.ok().map_err(|error| {
        platform(format!(
            "process loopback activation failed (requires Windows build 20348 or newer): {error}"
        ))
    })?;
    unknown
        .ok_or_else(|| platform("process loopback activation returned no interface"))?
        .cast()
        .map_err(platform)
}

struct CaptureLease<'a> {
    client: &'a IAudioCaptureClient,
    frames: u32,
    released: bool,
}
impl CaptureLease<'_> {
    fn release(mut self) -> Result<(), AudioError> {
        self.released = true;
        // SAFETY: this lease owns exactly the packet acquired on this thread.
        unsafe { self.client.ReleaseBuffer(self.frames) }.map_err(platform)
    }
}
impl Drop for CaptureLease<'_> {
    fn drop(&mut self) {
        if !self.released {
            // SAFETY: GetBuffer succeeded and this lease has not released it.
            let _ = unsafe { self.client.ReleaseBuffer(self.frames) };
        }
    }
}

/// Captures other processes' render streams, excluding SonaHop's process tree.
/// Requires Windows build 20348+ (including Windows 11). This is independent of
/// the playback endpoint and its master volume/mute. Create/use/drop on one MTA
/// audio worker; protected content may not be available for loopback capture.
pub struct SystemCapture {
    capture: IAudioCaptureClient,
    client: IAudioClient,
    event: Event,
    running: bool,
    // Rust destroys fields in declaration order; uninitialize COM last.
    _com: Apartment,
}
impl SystemCapture {
    pub fn new() -> Result<Self, AudioError> {
        let com = Apartment::new()?;
        let client = activate_process_loopback()?;
        let desired = wave_format();
        let flags = AUDCLNT_STREAMFLAGS_LOOPBACK
            | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
        // SAFETY: the complete format is live for Initialize. Process loopback
        // has no usable mix-format/buffer-size/padding query; do not call them.
        unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                flags,
                STREAM_BUFFER_HNS,
                0,
                desired.as_waveformatex_ref(),
                None,
            )
        }
        .map_err(platform)?;
        // SAFETY: this is an initialized capture stream.
        let capture = unsafe { client.GetService::<IAudioCaptureClient>() }.map_err(platform)?;
        let event = Event::new()?;
        // SAFETY: this owned event outlives the client. On error, release the
        // interfaces first, before the event handle and COM apartment unwind.
        if let Err(error) = unsafe { client.SetEventHandle(event.0) } {
            drop(capture);
            drop(client);
            return Err(platform(error));
        }
        let mut stream = Self {
            capture,
            client,
            event,
            running: false,
            _com: com,
        };
        // SAFETY: initialization and event registration succeeded on this thread.
        unsafe { stream.client.Start() }.map_err(platform)?;
        stream.running = true;
        Ok(stream)
    }
}
impl AudioCapture for SystemCapture {
    fn format(&self) -> AudioFormat {
        FORMAT
    }

    fn read_packet(&mut self, timeout: Duration) -> Result<Option<CapturePacket>, AudioError> {
        if !self.running {
            return Err(platform("capture is stopped"));
        }
        let began = Instant::now();
        loop {
            // SAFETY: capture and client are live on their MTA owner thread.
            if unsafe { self.capture.GetNextPacketSize() }.map_err(platform)? != 0 {
                break;
            }
            let remaining = timeout.saturating_sub(began.elapsed());
            if remaining.is_zero() {
                return Ok(None);
            }
            if !self.event.wait(remaining)? {
                return Ok(None);
            }
            // An event can be stale/spurious; recheck the actual packet size.
        }
        let mut data = std::ptr::null_mut();
        let mut frames = 0;
        let mut flags = 0;
        let mut position = 0;
        // SAFETY: all outputs are valid; Get/Release use the same thread.
        unsafe {
            self.capture.GetBuffer(
                &mut data,
                &mut frames,
                &mut flags,
                Some(&mut position),
                None,
            )
        }
        .map_err(platform)?;
        // Acquire the guard before every early return, including zero frames.
        let lease = CaptureLease {
            client: &self.capture,
            frames,
            released: false,
        };
        let byte_count = (frames as usize)
            .checked_mul(BYTES_PER_FRAME)
            .filter(|count| *count <= MAX_PACKET_BYTES)
            .ok_or(AudioError::InvalidPacket(
                "native audio packet exceeds 4 MiB",
            ))?;
        let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
        let bytes = if silent || byte_count == 0 {
            &[][..]
        } else {
            if data.is_null() {
                return Err(AudioError::InvalidPacket(
                    "non-silent native buffer is null",
                ));
            }
            // SAFETY: GetBuffer returned this complete frame count, it is bounded
            // above, and the guard retains the native buffer through decoding.
            unsafe { std::slice::from_raw_parts(data.cast_const(), byte_count) }
        };
        // Decode owns its samples before ReleaseBuffer. SILENT never reads data.
        let samples = decode_f32_packet(FORMAT, bytes, frames as usize, silent)?;
        lease.release()?;
        if frames == 0 {
            return Ok(None);
        }
        Ok(Some(CapturePacket {
            samples,
            position,
            discontinuity: flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0,
        }))
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if self.running {
            // SAFETY: the stream is used on its initialization thread.
            unsafe { self.client.Stop() }.map_err(platform)?;
            self.running = false;
        }
        Ok(())
    }
}
impl Drop for SystemCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct RenderLease<'a> {
    client: &'a IAudioRenderClient,
    released: bool,
}
impl RenderLease<'_> {
    fn release(mut self, frames: u32) -> Result<(), AudioError> {
        self.released = true;
        // SAFETY: all frames were copied into this thread's acquired buffer.
        unsafe { self.client.ReleaseBuffer(frames, 0) }.map_err(platform)
    }
}
impl Drop for RenderLease<'_> {
    fn drop(&mut self) {
        if !self.released {
            // SAFETY: releasing zero frames cancels this uncommitted shared-mode
            // write and prevents uninitialized samples from entering the queue.
            let _ = unsafe { self.client.ReleaseBuffer(0, 0) };
        }
    }
}

/// Plays interleaved float32, 48 kHz stereo into an explicitly selected render
/// endpoint, e.g. the render side of an existing virtual audio cable. WASAPI
/// shared-mode conversion handles the endpoint's native format/rate. No driver
/// installation or default-endpoint/master-volume changes are performed.
/// This type is !Send and !Sync; create/use/drop it on the same MTA audio worker.
pub struct AudioOutput {
    render: IAudioRenderClient,
    client: IAudioClient,
    event: Event,
    capacity: u32,
    running: bool,
    _com: Apartment,
}
impl AudioOutput {
    pub fn new(device_id: &str) -> Result<Self, AudioError> {
        if device_id.is_empty() || device_id.contains('\0') {
            return Err(platform("select a valid existing render endpoint ID"));
        }
        let com = Apartment::new()?;
        let enumerator = DeviceEnumerator::new().map_err(platform)?;
        let device = enumerator.get_device(device_id).map_err(platform)?;
        if device.get_direction() != Direction::Render {
            return Err(platform("select a render endpoint, not a microphone"));
        }
        // SAFETY: the apartment is initialized and class/interface agree.
        let raw_enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(platform)?;
        // SAFETY: ID has been checked and the temporary HSTRING lives through the call.
        let raw_device =
            unsafe { raw_enumerator.GetDevice(&HSTRING::from(device_id)) }.map_err(platform)?;
        // SAFETY: a render endpoint supports audio client activation.
        let client: IAudioClient =
            unsafe { raw_device.Activate(CLSCTX_ALL, None) }.map_err(platform)?;
        let desired = wave_format();
        let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
        // SAFETY: the input format is live for Initialize; shared-mode conversion
        // adapts float32/48k/stereo to the selected endpoint's native mix format.
        unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                flags,
                STREAM_BUFFER_HNS,
                0,
                desired.as_waveformatex_ref(),
                None,
            )
        }
        .map_err(platform)?;
        // SAFETY: regular render streams support the capacity query.
        let capacity = unsafe { client.GetBufferSize() }.map_err(platform)?;
        (capacity as usize)
            .checked_mul(BYTES_PER_FRAME)
            .filter(|count| *count > 0 && *count <= MAX_PACKET_BYTES)
            .ok_or(AudioError::InvalidFormat(
                "render buffer exceeds bounded capacity",
            ))?;
        // SAFETY: this initialized render stream exposes IAudioRenderClient.
        let render = unsafe { client.GetService::<IAudioRenderClient>() }.map_err(platform)?;
        // Release temporary COM objects before moving the apartment into the stream.
        drop(raw_device);
        drop(raw_enumerator);
        drop(device);
        drop(enumerator);
        let event = Event::new()?;
        // SAFETY: the event is retained for the entire stream. Release all
        // interfaces before closing this handle if registration fails.
        if let Err(error) = unsafe { client.SetEventHandle(event.0) } {
            drop(render);
            drop(client);
            return Err(platform(error));
        }
        let mut stream = Self {
            render,
            client,
            event,
            capacity,
            running: false,
            _com: com,
        };
        // SAFETY: initialization and event registration succeeded on this thread.
        unsafe { stream.client.Start() }.map_err(platform)?;
        stream.running = true;
        Ok(stream)
    }

    /// Returns consumed SAMPLES (always an even number), not frames. If the
    /// engine queue has no space by the deadline, returns 0. A partial write
    /// consumes only that prefix; the caller retries the remainder. The only
    /// queue is the bounded WASAPI buffer, with no growing software backlog.
    pub fn write(&mut self, samples: &[f32], timeout: Duration) -> Result<usize, AudioError> {
        if !self.running {
            return Err(platform("audio output is stopped"));
        }
        if samples.len() % CHANNELS != 0 {
            return Err(AudioError::InvalidPacket(
                "render samples must contain complete stereo frames",
            ));
        }
        if samples.is_empty() {
            return Ok(0);
        }
        let began = Instant::now();
        loop {
            // SAFETY: this regular render client is owned on the current thread.
            let padding = unsafe { self.client.GetCurrentPadding() }.map_err(platform)?;
            let available = self
                .capacity
                .checked_sub(padding)
                .ok_or(AudioError::InvalidPacket(
                    "render padding exceeds native capacity",
                ))?;
            if available != 0 {
                let frames = (samples.len() / CHANNELS).min(available as usize);
                let count = frames
                    .checked_mul(CHANNELS)
                    .ok_or(AudioError::InvalidPacket("render sample count overflow"))?;
                let bytes = count
                    .checked_mul(size_of::<f32>())
                    .filter(|count| *count <= MAX_PACKET_BYTES)
                    .ok_or(AudioError::InvalidPacket("render packet exceeds 4 MiB"))?;
                if samples[..count].iter().any(|sample| !sample.is_finite()) {
                    return Err(AudioError::InvalidPacket("non-finite render audio sample"));
                }
                // SAFETY: frames <= available <= capacity, and complete frame
                // bytes are bounded above. Get/Release remain on this thread.
                let data = unsafe { self.render.GetBuffer(frames as u32) }.map_err(platform)?;
                let lease = RenderLease {
                    client: &self.render,
                    released: false,
                };
                if data.is_null() {
                    return Err(AudioError::InvalidPacket("native render buffer is null"));
                }
                // SAFETY: native writable buffer has frames * 8 bytes. Byte copy
                // avoids imposing f32 alignment on WASAPI's pointer.
                unsafe {
                    std::ptr::copy_nonoverlapping(samples.as_ptr().cast::<u8>(), data, bytes)
                };
                lease.release(frames as u32)?;
                return Ok(count);
            }
            let remaining = timeout.saturating_sub(began.elapsed());
            if remaining.is_zero() {
                return Ok(0);
            }
            if !self.event.wait(remaining)? {
                return Ok(0);
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), AudioError> {
        if self.running {
            // SAFETY: this client is owned and used on its initialization thread.
            unsafe { self.client.Stop() }.map_err(platform)?;
            self.running = false;
        }
        // SAFETY: stream is stopped and no GetBuffer lease is outstanding.
        // Discard any queued microphone audio rather than replaying it later.
        unsafe { self.client.Reset() }.map_err(platform)
    }
}
impl Drop for AudioOutput {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
