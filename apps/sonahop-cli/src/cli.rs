use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "sonahop",
    version,
    about = "SonaHop Windows audio sender diagnostics"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List Windows playback devices without recording.
    Devices {
        #[arg(long)]
        json: bool,
    },
    /// Capture system playback into a diagnostic WAV, not a Windows microphone.
    Capture {
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..=60))]
        seconds: u64,
        #[arg(long)]
        device: Option<String>,
    },
}
