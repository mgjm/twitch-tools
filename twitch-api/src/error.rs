use std::fmt;

use indexmap::IndexMap;
use reqwest::StatusCode;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use thiserror::Error;

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("load config: {0}")]
    LoadConfig(#[source] toml::de::Error),

    #[error("save config: {0}")]
    SaveConfig(#[source] toml::ser::Error),

    #[error("send request: {0}")]
    SendRequest(#[source] reqwest::Error),

    #[error("parse response: {0}")]
    ParseReponse(#[source] reqwest::Error),

    #[error("parse error response: {0} {1}")]
    ParseErrorResponse(reqwest::StatusCode, #[source] reqwest::Error),

    #[error("error response: {0} {1}")]
    ErrorResponse(reqwest::StatusCode, ErrorResponse),

    #[error("unexpected api status: {0}")]
    UnexpectedApiStatus(reqwest::StatusCode),
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    #[serde(deserialize_with = "status_code")]
    pub status: StatusCode,

    pub message: String,

    #[serde(flatten)]
    pub data: IndexMap<String, Value>,
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.status, self.message)?;
        if !self.data.is_empty() {
            write!(f, " {:?}", self.data)?;
        }
        Ok(())
    }
}

fn status_code<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let status = u16::deserialize(deserializer)?;
    status.try_into().map_err(|_| {
        serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(status.into()),
            &"a valid status code",
        )
    })
}
