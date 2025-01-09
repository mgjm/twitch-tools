use serde::{Deserialize, Serialize};

use crate::{
    client::{Client, FormEncoding, Request},
    config::{ClientConfig, TokenConfig},
    error::Result,
    secret::Secret,
};

use super::TokenResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenManager {
    client_id: Secret,
    access_token: Secret,
    refresh_token: Secret,
}

impl TokenManager {
    pub fn from_env() -> Result<Self> {
        let client = ClientConfig::load_from_env()?;
        let token = TokenConfig::load_from_env()?;
        Ok(Self::with_config(client.client_id, token))
    }

    pub fn with_config(client_id: Secret, config: TokenConfig) -> Self {
        Self {
            client_id,
            access_token: config.access_token,
            refresh_token: config.refresh_token,
        }
    }
    pub fn access_token(&self) -> &Secret {
        &self.access_token
    }

    pub fn client_id(&self) -> &Secret {
        &self.client_id
    }

    fn config(&self) -> TokenConfig {
        TokenConfig {
            access_token: self.access_token.clone(),
            refresh_token: self.refresh_token.clone(),
        }
    }

    fn save(&self) -> Result<()> {
        self.config().save_to_env()
    }

    pub async fn update(&mut self, client: &mut Client) -> Result<()> {
        eprintln!("token manager: update access token");
        let res = client
            .send(&TokenRequest {
                client_id: self.client_id.clone(),
                grant_type: TokenRequest::GRANT_TYPE.into(),
                refresh_token: self.refresh_token.clone(),
            })
            .await?;
        self.access_token = res.access_token;
        self.refresh_token = res.refresh_token;
        self.save()
    }
}

#[derive(Debug, Serialize)]
pub struct TokenRequest {
    /// Your app’s client ID. See Registering your app.
    client_id: Secret,

    /// Your app’s client secret. See Registering your app.
    // client_secret: String,

    /// Must be set to `refresh_token`.
    grant_type: String,

    /// The refresh token issued to the client.
    ///
    /// You must URL encode the refresh token before posting the request.
    /// If you don’t, and the token contains restricted characters, the request may fail with “Invalid refresh token”.
    refresh_token: Secret,
}

impl TokenRequest {
    const GRANT_TYPE: &str = "refresh_token";
}

impl Request for TokenRequest {
    type Encoding = FormEncoding;
    type Response = TokenResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        "https://id.twitch.tv/oauth2/token"
    }
}
