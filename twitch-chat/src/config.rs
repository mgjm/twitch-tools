use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(rename = "output", default)]
    pub outputs: HashMap<String, OutputConfig>,

    #[serde(rename = "sound", default)]
    pub sounds: Vec<SoundConfig>,
}

impl Config {
    pub fn open(path: &Path) -> Result<Self> {
        let config = fs::read_to_string(path).context("read config file")?;
        toml::from_str(&config).context("parse config file")
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    #[serde(default)]
    pub device: Option<String>,

    #[serde(default)]
    pub volume: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundConfig {
    pub event: Event,

    pub sound: PathBuf,

    #[serde(default, deserialize_with = "vec_or_value")]
    pub output: Vec<String>,

    #[serde(default)]
    pub volume: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Message,
    Join,
    Leave,
    Follow,
    Online,
    Offline,
}

fn vec_or_value<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum VecOrValue<T> {
        Vec(Vec<T>),
        Value(T),
    }

    Ok(match VecOrValue::deserialize(deserializer)? {
        VecOrValue::Vec(vec) => vec,
        VecOrValue::Value(val) => {
            #[expect(clippy::vec_init_then_push)]
            {
                let mut vec = Vec::with_capacity(1);
                vec.push(val);
                vec
            }
        }
    })
}
