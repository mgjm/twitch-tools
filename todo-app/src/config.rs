use std::{collections::HashMap, fs, io, path::Path};

use anyhow::{Context, Result};
use crokey::KeyCombination;
use directories::ProjectDirs;
use serde::Deserialize;

use crate::model::Command;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "Keybindings::empty")]
    pub keybindings: Keybindings,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let config = fs::read_to_string(path)
            .or_else(|err| {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(String::new())
                } else {
                    Err(err)
                }
            })
            .context("read config")?;
        toml::from_str(&config).context("parse config")
    }

    pub fn load_env() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("de.mgjm", "twitch-tools", "todo-app")
            .context("failed to get config directory")?;
        let path = proj_dirs.config_dir().join("config.toml");
        Self::load(&path).with_context(|| format!("config: {path:?}"))
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Keybindings {
    #[serde(default)]
    pub normal: HashMap<KeyCombination, Command>,

    #[serde(default)]
    pub insert: HashMap<KeyCombination, Command>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            normal: Command::normal_keybindings().collect(),
            insert: Command::insert_keybindings().collect(),
        }
    }
}

impl Keybindings {
    pub fn empty() -> Self {
        Self {
            normal: HashMap::new(),
            insert: HashMap::new(),
        }
    }

    pub fn extend(&mut self, other: Self) {
        self.normal.extend(other.normal);
        self.insert.extend(other.insert);
    }
}
