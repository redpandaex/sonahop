//! Capture a recording endpoint on its owning MTA audio worker.
use super::OutputDevice;
use ::windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
        },
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance},
            Threading::{CreateEventW, WaitForSingleObject},
        },
    },
    core::{HSTRING, PCWSTR},
};
use sonahop_core::{
    AudioCapture, AudioError, AudioFormat, CapturePacket, MAX_PACKET_BYTES, decode_f32_packet,
};
use std::{
    marker::PhantomData,
    rc::Rc,
    time::{Duration, Instant},
};
use wasapi::{DeviceEnumerator, Direction, SampleType, WaveFormat};

const BYTES_PER_FRAME: usize = 8;
const FORMAT: AudioFormat = AudioFormat {
    sample_rate: 48_000,
    channels: 2,
    channel_mask: Some(3),
};

fn platform(error: impl std::fmt::Display) -> AudioError {
    AudioError::Platform(error.to_string())
}

fn wave_format() -> WaveFormat {
    WaveFormat::new(32, 32, &SampleType::Float, 48_000, 2, Some(3))
}

/// Prevents moving any of the native stream resources to another thread.
struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    fn new() -> Result<Self, AudioError> {
        wasapi::initialize_mta().ok().map_err(|error| {
            platform(format!(
                "microphone capture requires an MTA audio worker: {error}"
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
        let millis = remaining
            .as_millis()
            .saturating_add(u128::from(remaining.subsec_nanos() % 1_000_000 != 0))
            .min(u128::from(u32::MAX - 1)) as u32;
        // SAFETY: the owning stream keeps this event open through the wait.
        match unsafe { WaitForSingleObject(self.0, millis) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(platform(::windows::core::Error::from_thread())),
        }
    }
}
impl Drop for Event {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the event handle returned by CreateEventW.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct CaptureLease<'a> {
    client: &'a IAudioCaptureClient,
    frames: u32,
    released: bool,
}
impl CaptureLease<'_> {
    fn release(mut self) -> Result<(), AudioError> {
        self.released = true;
        // SAFETY: this thread acquired this exact packet with GetBuffer.
        unsafe { self.client.ReleaseBuffer(self.frames) }.map_err(platform)
    }
}
impl Drop for CaptureLease<'_> {
    fn drop(&mut self) {
        if !self.released {
            // SAFETY: the lease retains this packet through all error returns.
            let _ = unsafe { self.client.ReleaseBuffer(self.frames) };
        }
    }
}

/// Enumerate active recording endpoints without starting a stream. Only plain
/// device metadata crosses the thread boundary; all COM objects stay on the MTA.
pub fn list_input_devices() -> Result<Vec<OutputDevice>, AudioError> {
    std::thread::Builder::new()
        .name("sonahop-input-devices".into())
        .spawn(|| {
            let _com = Apartment::new()?;
            let enumerator = DeviceEnumerator::new().map_err(platform)?;
            let default_id = enumerator
                .get_default_device(&Direction::Capture)
                .ok()
                .and_then(|device| device.get_id().ok());
            let devices = enumerator
                .get_device_collection(&Direction::Capture)
                .map_err(platform)?;
            let mut inputs = Vec::new();
            for index in 0..devices.get_nbr_devices().map_err(platform)? {
                let device = devices.get_device_at_index(index).map_err(platform)?;
                let id = device.get_id().map_err(platform)?;
                let name = device.get_friendlyname().unwrap_or_else(|_| id.clone());
                inputs.push(OutputDevice {
                    is_default: default_id.as_ref() == Some(&id),
                    id,
                    name,
                });
            }
            Ok(inputs)
        })?
        .join()
        .map_err(|_| platform("recording-device enumeration worker panicked"))?
}

/// Capture a selected recording endpoint as float32, 48 kHz stereo. Create,
/// read, stop and drop inside the caller's newly created MTA audio worker.
/// This type is !Send and !Sync: it never transfers COM interfaces across threads.
/// Shared-mode WASAPI conversion adapts native mono/stereo and sample rates;
/// initialization fails explicitly if Windows cannot provide that conversion.
pub struct MicrophoneCapture {
    capture: IAudioCaptureClient,
    client: IAudioClient,
    event: Event,
    capacity: u32,
    running: bool,
    // Fields drop in declaration order: interfaces, event, then COM apartment.
    _com: Apartment,
}
impl MicrophoneCapture {
    pub fn new(device_id: Option<&str>) -> Result<Self, AudioError> {
        if device_id.is_some_and(|id| id.is_empty() || id.contains('\0')) {
            return Err(platform("select a valid recording endpoint ID"));
        }
        let com = Apartment::new()?;
        let enumerator = DeviceEnumerator::new().map_err(platform)?;
        let device = match device_id {
            Some(id) => enumerator.get_device(id).map_err(|error| {
                platform(format!("selected recording device is unavailable: {error}"))
            })?,
            None => enumerator
                .get_default_device(&Direction::Capture)
                .map_err(|error| {
                    platform(format!("default recording device is unavailable: {error}"))
                })?,
        };
        if device.get_direction() != Direction::Capture {
            return Err(platform(
                "select a recording endpoint, not a playback device",
            ));
        }
        let id = device.get_id().map_err(platform)?;
        // SAFETY: COM is initialized on this worker and class/interface agree.
        let raw_enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(platform)?;
        // SAFETY: the valid ID came from a recording endpoint; HSTRING outlives the call.
        let raw_device =
            unsafe { raw_enumerator.GetDevice(&HSTRING::from(id)) }.map_err(platform)?;
        // SAFETY: a recording endpoint exposes IAudioClient through activation.
        let client: IAudioClient =
            unsafe { raw_device.Activate(CLSCTX_ALL, None) }.map_err(platform)?;
        let desired = wave_format();
        let flags = AUDCLNT_STREAMFLAGS_EVENTCALLBACK
            | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
            | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
        // SAFETY: the complete desired format stays live through Initialize.
        // AUTOCONVERTPCM negotiates channel mixing and rate conversion, so the
        // client buffer is 48k float32 stereo even when the endpoint is not.
        // Shared/event-driven timing requires zero buffer duration and periodicity.
        unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                flags,
                0,
                0,
                desired.as_waveformatex_ref(),
                None,
            )
        }
        .map_err(|error| {
            platform(format!(
                "recording device cannot initialize 48 kHz float32 stereo: {error}"
            ))
        })?;
        // SAFETY: a regular endpoint capture client supports the capacity query.
        let capacity = unsafe { client.GetBufferSize() }.map_err(platform)?;
        (capacity as usize)
            .checked_mul(BYTES_PER_FRAME)
            .filter(|bytes| *bytes > 0 && *bytes <= MAX_PACKET_BYTES)
            .ok_or(AudioError::InvalidFormat(
                "microphone buffer exceeds bounded capacity",
            ))?;
        // SAFETY: this successfully initialized capture stream exposes this interface.
        let capture = unsafe { client.GetService::<IAudioCaptureClient>() }.map_err(platform)?;
        drop(raw_device);
        drop(raw_enumerator);
        drop(device);
        drop(enumerator);
        let event = Event::new()?;
        // SAFETY: the stream retains this event through native interface teardown.
        if let Err(error) = unsafe { client.SetEventHandle(event.0) } {
            drop(capture);
            drop(client);
            return Err(platform(error));
        }
        let mut stream = Self {
            capture,
            client,
            event,
            capacity,
            running: false,
            _com: com,
        };
        // SAFETY: initialization and event registration completed on the owner thread.
        unsafe { stream.client.Start() }
            .map_err(|error| platform(format!("recording device could not start: {error}")))?;
        stream.running = true;
        Ok(stream)
    }
}

/// SAFETY: non-silent data points to all frames acquired from GetBuffer, and
/// the caller retains the native lease until this function has decoded them.
unsafe fn decode_native_packet(
    data: *const u8,
    frames: u32,
    capacity: u32,
    flags: u32,
    position: u64,
) -> Result<CapturePacket, AudioError> {
    if frames > capacity {
        return Err(AudioError::InvalidPacket(
            "microphone packet exceeds native capacity",
        ));
    }
    let byte_count = (frames as usize)
        .checked_mul(BYTES_PER_FRAME)
        .filter(|bytes| *bytes <= MAX_PACKET_BYTES)
        .ok_or(AudioError::InvalidPacket("microphone packet exceeds 4 MiB"))?;
    let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
    let bytes = if silent || byte_count == 0 {
        &[][..]
    } else {
        if data.is_null() {
            return Err(AudioError::InvalidPacket(
                "non-silent microphone buffer is null",
            ));
        }
        // SAFETY: GetBuffer's complete frame count and the capacity are bounded;
        // the caller retains the lease, and byte access imposes no f32 alignment.
        unsafe { std::slice::from_raw_parts(data, byte_count) }
    };
    Ok(CapturePacket {
        samples: decode_f32_packet(FORMAT, bytes, frames as usize, silent)?,
        position,
        discontinuity: flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0,
    })
}

impl AudioCapture for MicrophoneCapture {
    fn format(&self) -> AudioFormat {
        FORMAT
    }

    fn read_packet(&mut self, timeout: Duration) -> Result<Option<CapturePacket>, AudioError> {
        if !self.running {
            return Err(platform("microphone capture is stopped"));
        }
        let began = Instant::now();
        loop {
            // SAFETY: this capture interface is live on its owning MTA thread.
            if unsafe { self.capture.GetNextPacketSize() }.map_err(platform)? != 0 {
                break;
            }
            let remaining = timeout.saturating_sub(began.elapsed());
            if remaining.is_zero() || !self.event.wait(remaining)? {
                return Ok(None);
            }
            // Recheck after stale/spurious events without extending the deadline.
        }
        let mut data = std::ptr::null_mut();
        let mut frames = 0;
        let mut flags = 0;
        let mut position = 0;
        // SAFETY: outputs are valid; GetBuffer and ReleaseBuffer stay on this thread.
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
        let lease = CaptureLease {
            client: &self.capture,
            frames,
            released: false,
        };
        // SAFETY: the lease retains GetBuffer's complete packet through decoding.
        let packet = unsafe {
            decode_native_packet(data.cast_const(), frames, self.capacity, flags, position)
        }?;
        lease.release()?;
        Ok((frames != 0).then_some(packet))
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if self.running {
            // SAFETY: the client remains on its initialization thread.
            unsafe { self.client.Stop() }.map_err(platform)?;
            self.running = false;
        }
        Ok(())
    }
}
impl Drop for MicrophoneCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_format_is_float32_48khz_stereo() {
        let format = wave_format();
        assert_eq!(format.get_samplespersec(), 48_000);
        assert_eq!(format.get_nchannels(), 2);
        assert_eq!(format.get_blockalign(), 8);
        assert_eq!(format.get_bitspersample(), 32);
        assert_eq!(format.get_validbitspersample(), 32);
        assert_eq!(format.get_dwchannelmask(), 3);
        assert!(matches!(format.get_subformat().unwrap(), SampleType::Float));
    }

    #[test]
    fn silent_null_packet_keeps_complete_stereo_frames() {
        let packet = unsafe {
            decode_native_packet(
                std::ptr::null(),
                2,
                2,
                (AUDCLNT_BUFFERFLAGS_SILENT.0 | AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0) as u32,
                17,
            )
        }
        .unwrap();
        assert_eq!(packet.samples, [0.0; 4]);
        assert_eq!(packet.position, 17);
        assert!(packet.discontinuity);
    }

    #[test]
    fn frame_capacity_is_checked_before_pointer_access() {
        assert!(unsafe { decode_native_packet(std::ptr::null(), 2, 1, 0, 0) }.is_err());
    }

    #[test]
    fn non_silent_null_buffer_is_rejected() {
        assert!(unsafe { decode_native_packet(std::ptr::null(), 1, 1, 0, 0) }.is_err());
    }

    #[test]
    fn decoded_packet_owns_its_samples() {
        let mut bytes = [0, 0, 0, 63, 0, 0, 0, 191];
        let packet = unsafe { decode_native_packet(bytes.as_ptr(), 1, 1, 0, 29) }.unwrap();
        bytes.fill(0);
        assert_eq!(packet.samples, [0.5, -0.5]);
        assert_eq!(packet.position, 29);
    }

    #[test]
    fn non_finite_native_sample_is_rejected() {
        let bytes = [0, 0, 192, 127, 0, 0, 0, 0];
        assert!(unsafe { decode_native_packet(bytes.as_ptr(), 1, 1, 0, 0) }.is_err());
    }
}
