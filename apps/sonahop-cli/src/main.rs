mod capture;
mod cli;

use clap::Parser;
use serde_json::json;
use sonahop_core::AudioError;
use sonahop_platform_windows::{list_output_devices, open_loopback};
use std::{
    io::BufWriter,
    process::ExitCode,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

fn run() -> Result<u8, AudioError> {
    match cli::Cli::parse().command {
        cli::Command::Devices { json: as_json } => {
            let devices = list_output_devices()?;
            if as_json {
                let values: Vec<_> = devices
                    .iter()
                    .map(|device| {
                        json!({
                            "id": device.id, "name": device.name, "is_default": device.is_default
                        })
                    })
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&values)
                        .map_err(|e| AudioError::Platform(e.to_string()))?
                );
            } else if devices.is_empty() {
                println!("No active Windows playback devices.");
            } else {
                for device in devices {
                    println!(
                        "{} {} [{}]",
                        if device.is_default { "*" } else { " " },
                        device.name,
                        device.id
                    );
                }
            }
            Ok(0)
        }
        cli::Command::Capture {
            output,
            seconds,
            device,
        } => {
            let file = capture::reserve_output(&output)?;
            let devices = list_output_devices()?;
            let selected = match device.as_deref() {
                Some(id) => devices.iter().find(|item| item.id == id),
                None => devices.iter().find(|item| item.is_default),
            }
            .ok_or_else(|| {
                AudioError::Platform("selected/default playback device is unavailable".into())
            })?;
            let cancelled = Arc::new(AtomicBool::new(false));
            let signal = cancelled.clone();
            ctrlc::set_handler(move || signal.store(true, std::sync::atomic::Ordering::Relaxed))
                .map_err(|e| AudioError::Platform(e.to_string()))?;
            let mut source = open_loopback(Some(&selected.id))?;
            let report = capture::capture_window(
                source.as_mut(),
                BufWriter::new(file),
                Duration::from_secs(seconds),
                &cancelled,
            )?;
            println!(
                "{}",
                json!({
                    "source": "windows-system-playback",
                    "device": { "id": selected.id, "name": selected.name },
                    "output": output,
                    "recording": report.recording,
                    "audio_duration_seconds": report.recording.duration_seconds(),
                    "window_elapsed_seconds": report.elapsed_seconds,
                    "packets": report.packets,
                    "discontinuities": report.discontinuities,
                    "zero_sample_frames": report.zero_sample_frames,
                    "cancelled": report.cancelled,
                })
            );
            Ok(if report.cancelled { 130 } else { 0 })
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
