use super::OutputDevice;
use ::windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY, AUDCLNT_BUFFERFLAGS_SILENT,
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK,
            AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioCaptureClient, IAudioClient,
            IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
        },
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree},
            Threading::{CreateEventW, WaitForSingleObject},
        },
    },
    core::{HSTRING, PCWSTR},
};
use sonahop_core::{
    AudioCapture, AudioError, AudioFormat, CapturePacket, MAX_PACKET_BYTES, decode_f32_packet,
};
use std::{marker::PhantomData, rc::Rc, time::Duration};
use wasapi::{DeviceEnumerator, Direction, SampleType, WaveFormat};

fn platform(error: impl std::fmt::Display) -> AudioError {
    AudioError::Platform(error.to_string())
}

/// Keeps initialization, use and release on the same thread.
struct Apartment(PhantomData<Rc<()>>);
impl Apartment {
    fn new() -> Result<Self, AudioError> {
        wasapi::initialize_mta().ok().map_err(platform)?;
        Ok(Self(PhantomData))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        wasapi::deinitialize();
    }
}

struct Event(HANDLE);
impl Drop for Event {
    fn drop(&mut self) {
        // SAFETY: this object owns the handle returned by CreateEventW.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct MixAllocation(*mut WAVEFORMATEX);
impl Drop for MixAllocation {
    fn drop(&mut self) {
        // SAFETY: GetMixFormat allocates this value with CoTaskMemAlloc.
        unsafe { CoTaskMemFree(Some(self.0.cast())) };
    }
}

struct BufferLease<'a> {
    client: &'a IAudioCaptureClient,
    frames: u32,
    released: bool,
}
impl BufferLease<'_> {
    fn release(mut self) -> Result<(), AudioError> {
        self.released = true;
        // SAFETY: the same thread acquired exactly this complete packet.
        unsafe { self.client.ReleaseBuffer(self.frames) }.map_err(platform)
    }
}
impl Drop for BufferLease<'_> {
    fn drop(&mut self) {
        if !self.released {
            // SAFETY: ownership remains with this lease until released.
            let _ = unsafe { self.client.ReleaseBuffer(self.frames) };
        }
    }
}

pub fn list_output_devices() -> Result<Vec<OutputDevice>, AudioError> {
    let _com = Apartment::new()?;
    let enumerator = DeviceEnumerator::new().map_err(platform)?;
    let default_id = enumerator
        .get_default_device(&Direction::Render)
        .ok()
        .and_then(|device| device.get_id().ok());
    let devices = enumerator
        .get_device_collection(&Direction::Render)
        .map_err(platform)?;
    let mut outputs = Vec::new();
    for index in 0..devices.get_nbr_devices().map_err(platform)? {
        let device = devices.get_device_at_index(index).map_err(platform)?;
        let id = device.get_id().map_err(platform)?;
        let name = device.get_friendlyname().unwrap_or_else(|_| id.clone());
        outputs.push(OutputDevice {
            is_default: default_id.as_ref() == Some(&id),
            id,
            name,
        });
    }
    Ok(outputs)
}

fn format_from_mix(mix: &WaveFormat) -> Result<AudioFormat, AudioError> {
    AudioFormat::new(mix.get_samplespersec(), mix.get_nchannels())
}

/// Copy while the native buffer is borrowed. SILENT never dereferences data.
/// SAFETY: for non-silent data the pointer must refer to the complete packet
/// obtained from GetBuffer, and its lease must not have been released.
unsafe fn copy_packet(
    format: AudioFormat,
    data: *const u8,
    frames: u32,
    capacity: u32,
    flags: u32,
    position: u64,
    scratch: &mut [u8],
) -> Result<CapturePacket, AudioError> {
    if frames > capacity {
        return Err(AudioError::InvalidPacket(
            "device packet exceeds buffer capacity",
        ));
    }
    let byte_count = (frames as usize)
        .checked_mul(format.bytes_per_frame()?)
        .filter(|size| *size <= MAX_PACKET_BYTES)
        .ok_or(AudioError::InvalidPacket(
            "native audio packet exceeds bounds",
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
        if byte_count > scratch.len() {
            return Err(AudioError::InvalidPacket(
                "native packet exceeds allocated scratch buffer",
            ));
        }
        // SAFETY: pointer validity is the caller's contract; size is bounded
        // by the complete native packet and the allocated scratch buffer.
        let native = unsafe { std::slice::from_raw_parts(data, byte_count) };
        scratch[..byte_count].copy_from_slice(native);
        &scratch[..byte_count]
    };
    let samples = decode_f32_packet(format, bytes, frames as usize, silent)?;
    Ok(CapturePacket {
        samples,
        position,
        discontinuity: flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0,
    })
}

struct Loopback {
    capture: IAudioCaptureClient,
    client: IAudioClient,
    event: Event,
    format: AudioFormat,
    capacity: u32,
    scratch: Vec<u8>,
    running: bool,
    // Fields are destroyed in declaration order, so COM is released last.
    _com: Apartment,
}

impl AudioCapture for Loopback {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn read_packet(&mut self, timeout: Duration) -> Result<Option<CapturePacket>, AudioError> {
        if !self.running {
            return Err(AudioError::Platform("capture is stopped".into()));
        }
        // SAFETY: the apartment and stream are active on this thread.
        if unsafe { self.capture.GetNextPacketSize() }.map_err(platform)? == 0 {
            let milliseconds = timeout.as_millis().min(u128::from(u32::MAX - 1)) as u32;
            // SAFETY: this stream owns a live event handle.
            match unsafe { WaitForSingleObject(self.event.0, milliseconds) } {
                WAIT_TIMEOUT => return Ok(None),
                WAIT_OBJECT_0 => {}
                _ => return Err(platform(::windows::core::Error::from_thread())),
            }
        }
        let mut data = std::ptr::null_mut();
        let mut frames = 0;
        let mut flags = 0;
        let mut position = 0;
        // SAFETY: all out pointers are valid and Get/Release stay on this thread.
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
        if frames == 0 {
            return Ok(None);
        }
        let lease = BufferLease {
            client: &self.capture,
            frames,
            released: false,
        };
        // SAFETY: the lease owns this complete packet until after the copy.
        let packet = unsafe {
            copy_packet(
                self.format,
                data,
                frames,
                self.capacity,
                flags,
                position,
                &mut self.scratch,
            )
        };
        lease.release()?;
        packet.map(Some)
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        if self.running {
            // SAFETY: stream is owned and used on its initialization thread.
            unsafe { self.client.Stop() }.map_err(platform)?;
            self.running = false;
        }
        Ok(())
    }
}
impl Drop for Loopback {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub fn open_loopback(device_id: Option<&str>) -> Result<Box<dyn AudioCapture>, AudioError> {
    let com = Apartment::new()?;
    let enumerator = DeviceEnumerator::new().map_err(platform)?;
    let device = match device_id {
        Some(id) => enumerator.get_device(id).map_err(platform)?,
        None => enumerator
            .get_default_device(&Direction::Render)
            .map_err(platform)?,
    };
    if device.get_direction() != Direction::Render {
        return Err(AudioError::Platform(
            "select a playback device, not a microphone".into(),
        ));
    }
    let id = device.get_id().map_err(platform)?;
    // SAFETY: COM is initialized and the requested class/interface match.
    let raw_enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.map_err(platform)?;
    // SAFETY: HSTRING lives through GetDevice; the ID came from a Render endpoint.
    let raw_device = unsafe { raw_enumerator.GetDevice(&HSTRING::from(id)) }.map_err(platform)?;
    // SAFETY: this endpoint supports IAudioClient activation.
    let client: IAudioClient =
        unsafe { raw_device.Activate(CLSCTX_ALL, None) }.map_err(platform)?;
    // SAFETY: GetMixFormat returns a COM-allocated format owned by allocation.
    let allocation = MixAllocation(unsafe { client.GetMixFormat() }.map_err(platform)?);
    if allocation.0.is_null() {
        return Err(AudioError::InvalidFormat("native mix format is null"));
    }
    // SAFETY: the returned format contains at least the base header.
    let header = unsafe { std::ptr::read_unaligned(allocation.0) };
    let length = std::mem::size_of::<WAVEFORMATEX>() + usize::from(header.cbSize);
    if length > std::mem::size_of::<WAVEFORMATEXTENSIBLE>() {
        return Err(AudioError::InvalidFormat(
            "unsupported native format extension",
        ));
    }
    // SAFETY: GetMixFormat allocates the header and its declared extension.
    let mix_bytes = unsafe { std::slice::from_raw_parts(allocation.0.cast::<u8>(), length) };
    let mix = WaveFormat::parse_from_blob_bytes(mix_bytes).map_err(platform)?;
    let format = format_from_mix(&mix)?;
    let desired = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        format.sample_rate as usize,
        usize::from(format.channels),
        None,
    );
    let flags = AUDCLNT_STREAMFLAGS_LOOPBACK
        | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
        | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
    // SAFETY: the complete float format stays live for Initialize.
    unsafe {
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            flags,
            1_000_000,
            0,
            desired.as_waveformatex_ref(),
            None,
        )
    }
    .map_err(platform)?;
    // SAFETY: creates an unnamed, auto-reset event owned by Event.
    let event =
        Event(unsafe { CreateEventW(None, false, false, PCWSTR::null()) }.map_err(platform)?);
    // SAFETY: the event remains alive for the entire stream.
    unsafe { client.SetEventHandle(event.0) }.map_err(platform)?;
    let capacity = unsafe { client.GetBufferSize() }.map_err(platform)?;
    let buffer_bytes = (capacity as usize)
        .checked_mul(format.bytes_per_frame()?)
        .filter(|size| *size > 0 && *size <= MAX_PACKET_BYTES)
        .ok_or(AudioError::InvalidFormat(
            "native buffer size exceeds bounds",
        ))?;
    let capture: IAudioCaptureClient = unsafe { client.GetService() }.map_err(platform)?;
    drop(allocation);
    drop(raw_device);
    drop(raw_enumerator);
    drop(device);
    drop(enumerator);
    let mut loopback = Loopback {
        capture,
        client,
        event,
        format,
        capacity,
        scratch: vec![0; buffer_bytes],
        running: false,
        _com: com,
    };
    // SAFETY: the client is fully initialized and owned by this object.
    unsafe { loopback.client.Start() }.map_err(platform)?;
    loopback.running = true;
    Ok(Box::new(loopback))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_format_preserves_rate_and_all_channels() {
        let mix = WaveFormat::new(32, 32, &SampleType::Float, 44_100, 6, None);
        assert_eq!(
            format_from_mix(&mix).unwrap(),
            AudioFormat {
                sample_rate: 44_100,
                channels: 6
            }
        );
    }
    #[test]
    fn silent_null_buffer_preserves_frame_count_and_metadata() {
        let packet = unsafe {
            copy_packet(
                AudioFormat::new(48_000, 2).unwrap(),
                std::ptr::null(),
                1,
                4,
                3,
                123,
                &mut [],
            )
        }
        .unwrap();
        assert_eq!(packet.samples, [0.0, 0.0]);
        assert_eq!(packet.position, 123);
        assert!(packet.discontinuity);
    }
    #[test]
    fn frame_count_over_capacity_is_rejected_before_pointer_access() {
        assert!(
            unsafe {
                copy_packet(
                    AudioFormat::new(48_000, 2).unwrap(),
                    std::ptr::null(),
                    2,
                    1,
                    0,
                    0,
                    &mut [0; 8],
                )
            }
            .is_err()
        );
    }
    #[test]
    fn non_silent_null_pointer_is_rejected() {
        assert!(
            unsafe {
                copy_packet(
                    AudioFormat::new(48_000, 2).unwrap(),
                    std::ptr::null(),
                    1,
                    4,
                    0,
                    0,
                    &mut [0; 8],
                )
            }
            .is_err()
        );
    }
    #[test]
    fn real_buffer_is_copied_before_it_is_released() {
        let mut data = [0, 0, 0, 63, 0, 0, 0, 191];
        let packet = unsafe {
            copy_packet(
                AudioFormat::new(48_000, 2).unwrap(),
                data.as_ptr(),
                1,
                1,
                1,
                77,
                &mut [0; 8],
            )
        }
        .unwrap();
        data.fill(0);
        assert_eq!(packet.samples, [0.5, -0.5]);
        assert_eq!(packet.position, 77);
        assert!(packet.discontinuity);
    }
    #[test]
    fn undersized_scratch_buffer_is_rejected() {
        let data = [0u8; 8];
        assert!(
            unsafe {
                copy_packet(
                    AudioFormat::new(48_000, 2).unwrap(),
                    data.as_ptr(),
                    1,
                    1,
                    0,
                    0,
                    &mut [0; 4],
                )
            }
            .is_err()
        );
    }
}
