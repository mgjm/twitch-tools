use std::io;

use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::{
    client::{Client, FormEncoding, Request},
    config::{ClientConfig, TokenConfig},
    secret::Secret,
};

mod token_manager;

pub use self::token_manager::TokenManager;

#[derive(Debug, Args)]
/// Authorize client against twitch api
pub struct Auth {}

impl Auth {
    pub async fn run(self, scopes: impl IntoIterator<Item = Scope>) -> Result<()> {
        let config = ClientConfig::load_from_env()?;
        eprintln!("{config:#?}");

        let scopes = Scopes::from_iter(scopes);

        let client = Client::new();

        let res = client
            .send(&DeviceRequest {
                client_id: config.client_id.clone(),
                scopes: scopes.clone(),
            })
            .await
            .context("device request")?;

        eprintln!("{res:#?}");
        println!("{}", res.verification_uri.access_secret_value());

        {
            eprint!("Press ENTER once authenticated using the provided URL: ");
            let mut buf = String::new();
            let result = io::stdin().read_line(&mut buf);
            let nl = buf.ends_with("\n");
            if !nl {
                eprintln!();
            }
            result.context("receive ENTER from stdin")?;
            anyhow::ensure!(nl, "authentication canceled");
        }

        eprintln!("Ok");

        let res = client
            .send(&TokenRequest {
                client_id: config.client_id,
                scopes,
                device_code: res.device_code,
                grant_type: TokenRequest::GRANT_TYPE.into(),
            })
            .await
            .context("token request")?;

        eprintln!("{res:#?}");

        TokenConfig {
            access_token: res.access_token,
            refresh_token: res.refresh_token,
        }
        .save_to_env()
        .context("save tokens")?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct DeviceRequest {
    /// Your app’s registered Client ID.
    client_id: Secret,

    /// A space-delimited list of scopes. The APIs that you’re calling identify the scopes you must list. You must URL encode the list.
    scopes: Scopes,
}

impl Request for DeviceRequest {
    type Encoding = FormEncoding;
    type Response = DeviceResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        "https://id.twitch.tv/oauth2/device"
    }
}

#[derive(Debug, Deserialize)]
pub struct DeviceResponse {
    /// The identifier for a given user.
    pub device_code: Secret,

    /// Time until the code is no longer valid
    pub expires_in: u32,

    /// Time until another valid code can be requested
    pub interval: u32,

    /// The code that the user will use to authenticate
    pub user_code: Secret,

    /// The address you will send users to, to authenticate
    pub verification_uri: Secret,
}

#[derive(Debug, Serialize)]
pub struct TokenRequest {
    /// Your app’s registered client ID.
    client_id: Secret,

    /// A space-delimited list of scopes. The APIs that you’re calling identify the scopes you must list. You must URL encode the list.
    scopes: Scopes,

    /// The code that will authenticate the user.
    device_code: Secret,

    /// Must be set to `urn:ietf:params:oauth:grant-type:device_code`.
    grant_type: String,
}

impl TokenRequest {
    const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
}

impl Request for TokenRequest {
    type Encoding = FormEncoding;
    type Response = TokenResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        "https://id.twitch.tv/oauth2/token"
    }
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    /// The authenticated token, to be used for various API endpoints and EventSub subscriptions.
    pub access_token: Secret,

    /// Time until the code is no longer valid.
    pub expires_in: u32,

    /// A token used to refresh the access token.
    pub refresh_token: Secret,

    /// An array of the scopes requested.
    pub scope: Vec<Scope>,

    /// Will generally be "beare"
    pub token_type: String,
}

#[derive(Debug, Clone)]
pub struct Scopes(Vec<Scope>);

impl FromIterator<Scope> for Scopes {
    fn from_iter<T: IntoIterator<Item = Scope>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Serialize for Scopes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut scopes = self.0.iter().map(|s| s.to_str());
        let mut s = scopes.next().unwrap_or_default().to_string();
        for scope in scopes {
            s.push(' ');
            s.push_str(scope);
        }
        s.serialize(serializer)
    }
}

macro_rules! scopes {
    ($($ident:ident => $str:literal,)*) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
        pub enum Scope {
            $(
                #[serde(rename=$str)]
                $ident,
            )*
        }

        impl Scope {
            fn to_str(self) -> &'static str {
                match self {
                    $(Self::$ident => $str,)*
                }
            }
        }
    };
}

scopes! {
    UserReadChat => "user:read:chat",
    UserWriteChat => "user:write:chat",
    ModeratorManageAnnouncements => "moderator:manage:announcements",
    ModeratorReadFollowers => "moderator:read:followers",
}
