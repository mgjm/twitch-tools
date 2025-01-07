use std::fmt;

use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&"*".repeat(self.0.len()))
    }
}

impl TryFrom<Secret> for HeaderValue {
    type Error = <HeaderValue as TryFrom<String>>::Error;

    fn try_from(value: Secret) -> Result<Self, Self::Error> {
        format!("Bearer {}", value.0).try_into()
    }
}
