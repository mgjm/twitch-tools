use std::{io, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use sound_fx_3000::{Output, Sound};

#[derive(Debug, Parser)]
#[clap(version)]
/// Example audio player
struct Args {
    #[clap(long)]
    /// Pulse audio output device
    device: Option<String>,

    #[clap(long)]
    /// Output volume
    volume: Option<f32>,

    /// Path to an audio file
    path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    eprintln!("{args:#?}");

    let mut sound = Sound::open(&args.path)?;

    if let Some(volume) = args.volume {
        sound.set_volume(volume);
    }

    eprintln!("start");

    let output = Output::spawn(sound.spec().rate, args.device.as_deref())?;

    for line in io::stdin().lines() {
        let _line = line.context("read line")?;
        output.play(&sound)?;
    }

    eprintln!("done");
    Ok(())
}
