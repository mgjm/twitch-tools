use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, Error as _},
    ser::Error as _,
};

use crate::{
    error::{ApiError, Result},
    secret::Secret,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientConfig {
    pub client_id: Secret,
}

impl ClientConfig {
    pub fn load(path: &Path) -> Result<Self> {
        load_toml(path)
    }

    pub(crate) fn load_from_env() -> Result<Self> {
        Self::load(&from_env("TWITCH_CLIENT_CONFIG", "client-config.toml"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenConfig {
    pub access_token: Secret,
    pub refresh_token: Secret,
}

impl TokenConfig {
    pub fn load(path: &Path) -> Result<Self> {
        load_toml(path)
    }

    fn env() -> PathBuf {
        from_env("TWITCH_CLIENT_CONFIG", "client-config.toml")
    }

    pub(crate) fn load_from_env() -> Result<Self> {
        Self::load(&Self::env())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        save_toml(path, self)
    }

    pub fn save_to_env(&self) -> Result<()> {
        self.save(&Self::env())
    }
}

fn from_env(key: &str, default_value: &str) -> PathBuf {
    env::var_os(key)
        .unwrap_or_else(|| default_value.into())
        .into()
}

fn load_toml<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let config = fs::read_to_string(path)
        .map_err(toml::de::Error::custom)
        .map_err(ApiError::LoadConfig)?;
    toml::from_str(&config).map_err(ApiError::LoadConfig)
}

fn save_toml(path: &Path, config: &impl Serialize) -> Result<()> {
    let config = toml::to_string(config).map_err(ApiError::SaveConfig)?;

    fs::write(path, config)
        .map_err(toml::ser::Error::custom)
        .map_err(ApiError::SaveConfig)
}
