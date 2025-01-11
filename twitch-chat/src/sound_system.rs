use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use sound_fx_3000::{Output, Sound};

use crate::config::{Event, OutputConfig, SoundConfig};

pub(crate) struct SoundSystem {
    pub(crate) outputs: HashMap<String, Output>,
    pub(crate) sounds: HashMap<Event, Vec<(String, Sound)>>,
}

impl SoundSystem {
    pub fn init(
        mut outputs: HashMap<String, OutputConfig>,
        sounds: Vec<SoundConfig>,
    ) -> Result<Self> {
        let mut sample_rate = None;

        let mut this = Self {
            outputs: Default::default(),
            sounds: Default::default(),
        };

        pub(crate) const DEFAULT_NAME: &str = "default";
        if !outputs.contains_key(DEFAULT_NAME) {
            outputs.insert(DEFAULT_NAME.into(), OutputConfig {
                device: None,
                volume: None,
            });
        }

        let mut used_outputs = HashSet::new();

        for mut sound_config in sounds {
            let mut sound = Sound::open(&sound_config.sound)?;
            if let Some(volume) = sound_config.volume {
                sound.set_volume(volume);
            }
            if let Some(sample_rate) = sample_rate {
                anyhow::ensure!(
                    sample_rate == sound.spec().rate,
                    "sample rate does not match: {} != {}",
                    sample_rate,
                    sound.spec().rate,
                )
            } else {
                sample_rate = Some(sound.spec().rate);
            }
            if sound_config.output.is_empty() {
                sound_config.output.push(DEFAULT_NAME.into());
            }
            for output in sound_config.output {
                used_outputs.insert(output.clone());

                let mut sound = sound.clone();
                if let Some(volume) = outputs
                    .get(&output)
                    .with_context(|| format!("unknown sound output: {output:?}"))?
                    .volume
                {
                    sound.set_volume(volume);
                }
                this.sounds
                    .entry(sound_config.event)
                    .or_default()
                    .push((output, sound));
            }
        }

        if let Some(sample_rate) = sample_rate {
            for (name, output_config) in outputs {
                if !used_outputs.contains(&name) {
                    continue;
                }
                let output = Output::spawn(sample_rate, output_config.device.as_deref())?;
                this.outputs.insert(name, output);
            }
        }

        Ok(this)
    }

    pub(crate) fn play_sound_for_event(&mut self, event: Event) {
        for (output, sound) in self.sounds.get(&event).into_iter().flatten() {
            let Some(output) = self.outputs.get(output) else {
                continue;
            };
            if let Err(err) = output.play(sound) {
                eprintln!("failed to play sound for {event:?}: {err:?}");
            }
        }
    }
}
