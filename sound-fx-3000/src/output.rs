use std::{
    any::Any,
    sync::{mpsc, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use libpulse_binding::{
    channelmap::{Map as ChannelMap, Position},
    sample::{Format, Spec},
    stream::Direction,
};
use libpulse_simple_binding::Simple;
use symphonia::core::audio::Channels;
use zerocopy::IntoBytes;

use crate::Sound;

type Frames = Arc<[[f32; 2]]>;

const CHUNK_SIZE: usize = 1024;

/// Handle to play sounds
///
/// An output thread gets spawnd and the handle can be used to submit sounds.
pub struct Output {
    sample_rate: u32,
    tx: mpsc::Sender<Frames>,
    handle: JoinHandle<()>,
}

impl Output {
    /// Spawn the output thread and return the output handle
    pub fn spawn(sample_rate: u32, device: Option<&str>) -> Result<Self> {
        let output = PaOutput::open(sample_rate, device)?;

        let (tx, rx) = mpsc::channel();

        let handle = std::thread::Builder::new()
            .name("audio output".into())
            .spawn(move || {
                run(sample_rate, output, rx);
            })
            .context("spawn audio output thread")?;

        Ok(Self {
            sample_rate,
            tx,
            handle,
        })
    }

    /// Play a sound by submitting it to the worker thread
    pub fn play(&self, sound: &Sound) -> Result<()> {
        anyhow::ensure!(
            sound.spec().rate == self.sample_rate,
            "sample rate does not match: expected {}, got {}",
            self.sample_rate,
            sound.spec().rate,
        );
        self.tx.send(sound.frames()).context("start sound")?;
        Ok(())
    }

    /// Stop the worker thread after all remaining sound is played
    pub fn shutdown(self) -> Result<()> {
        drop(self.tx);
        match self.handle.join() {
            Ok(()) => Ok(()),
            Err(err) => {
                anyhow::bail!("audio output thread panicked: {}", payload_as_str(&err));
            }
        }
    }
}

fn run(sample_rate: u32, mut output: PaOutput, rx: mpsc::Receiver<Frames>) {
    let mut playing = Vec::new();
    let mut start = Instant::now();
    loop {
        if playing.is_empty() {
            let Ok(sound) = rx.recv() else { break };
            playing.push((sound, 0));
            start = Instant::now();
        } else if let Ok(sound) = rx.try_recv() {
            playing.push((sound, 0));
        }

        let mut chunk = [[0.0; 2]; CHUNK_SIZE];
        for (sound, index) in &mut playing {
            let sound_chunk = &sound[*index..];
            let sound_chunk = sound_chunk.get(..chunk.len()).unwrap_or(sound_chunk);
            for (c, s) in std::iter::zip(&mut chunk, sound_chunk) {
                c[0] += s[0];
                c[1] += s[1];
            }
            *index += chunk.len();
        }
        playing.retain(|(sound, index)| *index < sound.len());

        output.write(&chunk).unwrap();
        start += Duration::from_secs(chunk.len() as u64) / sample_rate;
        if let Some(delay) = start.checked_duration_since(Instant::now()) {
            std::thread::sleep(delay);
        }
    }
}

struct PaOutput {
    pa: Simple,
}

impl PaOutput {
    fn open(sample_rate: u32, device: Option<&str>) -> Result<Self> {
        let pa_spec = Spec {
            format: Format::FLOAT32NE,
            rate: sample_rate,
            channels: 2,
        };

        anyhow::ensure!(pa_spec.is_valid(), "pulse audio spec invalid");

        let pa_ch_map =
            map_channels_to_pa_channelmap(Channels::FRONT_LEFT | Channels::FRONT_RIGHT)?;

        let pa = Simple::new(
            None,
            "twitch-tools",
            Direction::Playback,
            device,
            "twitch-tools-sounds",
            &pa_spec,
            Some(&pa_ch_map),
            None,
        )
        .context("open audio output")?;

        Ok(Self { pa })
    }

    fn write(&mut self, data: &[[f32; 2]]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        self.pa
            .write(data.as_bytes())
            .context("write to audio output")
    }
}

fn map_channels_to_pa_channelmap(channels: Channels) -> Result<ChannelMap> {
    let mut map = ChannelMap::default();
    map.init();
    map.set_len(channels.count() as u8);

    let is_mono = channels.count() == 1;

    for (channel, position) in channels.iter().zip(map.get_mut()) {
        *position = match channel {
            Channels::FRONT_LEFT if is_mono => Position::Mono,
            Channels::FRONT_LEFT => Position::FrontLeft,
            Channels::FRONT_RIGHT => Position::FrontRight,
            Channels::FRONT_CENTRE => Position::FrontCenter,
            Channels::REAR_LEFT => Position::RearLeft,
            Channels::REAR_CENTRE => Position::RearCenter,
            Channels::REAR_RIGHT => Position::RearRight,
            Channels::LFE1 => Position::Lfe,
            Channels::FRONT_LEFT_CENTRE => Position::FrontLeftOfCenter,
            Channels::FRONT_RIGHT_CENTRE => Position::FrontRightOfCenter,
            Channels::SIDE_LEFT => Position::SideLeft,
            Channels::SIDE_RIGHT => Position::SideRight,
            Channels::TOP_CENTRE => Position::TopCenter,
            Channels::TOP_FRONT_LEFT => Position::TopFrontLeft,
            Channels::TOP_FRONT_CENTRE => Position::TopFrontCenter,
            Channels::TOP_FRONT_RIGHT => Position::TopFrontRight,
            Channels::TOP_REAR_LEFT => Position::TopRearLeft,
            Channels::TOP_REAR_CENTRE => Position::TopRearCenter,
            Channels::TOP_REAR_RIGHT => Position::TopRearRight,
            _ => {
                anyhow::bail!("failed to map unknown channel: {channel}");
            }
        }
    }

    Ok(map)
}

fn payload_as_str(payload: &dyn Any) -> &str {
    if let Some(&s) = payload.downcast_ref::<&'static str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "Box<dyn Any>"
    }
}
