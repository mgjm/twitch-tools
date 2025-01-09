use std::fmt;

use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl ToString) -> Self {
        Self(value.to_string())
    }

    pub fn access_secret_value(&self) -> &str {
        &self.0
    }

    pub fn bearer(&self) -> Bearer {
        Bearer(self)
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&"*".repeat(self.0.len()))
    }
}

impl TryFrom<&Secret> for HeaderValue {
    type Error = <HeaderValue as TryFrom<String>>::Error;

    fn try_from(value: &Secret) -> Result<Self, Self::Error> {
        value.0.as_str().try_into()
    }
}

pub struct Bearer<'a>(&'a Secret);

impl TryFrom<Bearer<'_>> for HeaderValue {
    type Error = <HeaderValue as TryFrom<String>>::Error;

    fn try_from(value: Bearer) -> Result<Self, Self::Error> {
        format!("Bearer {}", value.0.access_secret_value()).try_into()
    }
}
