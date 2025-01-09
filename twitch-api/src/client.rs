use reqwest::{IntoUrl, Method, RequestBuilder, StatusCode, header};
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    auth::TokenManager,
    error::{ApiError, ErrorResponse, Result},
    secret::Secret,
};

pub struct AuthenticatedClient {
    client: Client,
    token_manager: TokenManager,
}

impl AuthenticatedClient {
    pub async fn send<T>(&mut self, req: &T) -> Result<T::Response>
    where
        T: Request,
    {
        match self
            .client
            .send_inner(
                req,
                Some((
                    self.token_manager.access_token(),
                    self.token_manager.client_id(),
                )),
            )
            .await
        {
            Err(ApiError::ErrorResponse(StatusCode::UNAUTHORIZED, res))
                if res.status == StatusCode::UNAUTHORIZED =>
            {
                self.token_manager.update(&mut self.client).await?;
                self.client
                    .send_inner(
                        req,
                        Some((
                            self.token_manager.access_token(),
                            self.token_manager.client_id(),
                        )),
                    )
                    .await
            }
            res => res,
        }
    }
}

pub struct Client {
    client: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn authenticated(self, token_manager: TokenManager) -> AuthenticatedClient {
        AuthenticatedClient {
            client: self,
            token_manager,
        }
    }

    pub fn authenticated_from_env(self) -> Result<AuthenticatedClient> {
        Ok(self.authenticated(TokenManager::from_env()?))
    }

    pub async fn send<T>(&self, req: &T) -> Result<T::Response>
    where
        T: Request,
    {
        self.send_inner(req, None).await
    }

    async fn send_inner<T>(
        &self,
        req: &T,
        access_token_and_client_id: Option<(&Secret, &Secret)>,
    ) -> Result<T::Response>
    where
        T: Request,
    {
        let res = self
            .client
            .request(T::Encoding::METHOD, req.url())
            .encode(req)
            .access_token_and_client_id(access_token_and_client_id)
            .send()
            .await
            .map_err(ApiError::SendRequest)?;

        let status = res.status();

        if status.is_success() {
            res.json::<T::Response>()
                .await
                .map_err(ApiError::ParseReponse)
        } else if status.is_client_error() || status.is_server_error() {
            let res = res
                .json::<ErrorResponse>()
                .await
                .map_err(|err| ApiError::ParseErrorResponse(status, err))?;
            Err(ApiError::ErrorResponse(status, res))
        } else {
            Err(ApiError::UnexpectedApiStatus(status))
        }
    }
}

trait RequestBuilderExt {
    fn encode<T>(self, req: &T) -> Self
    where
        T: Request;

    fn access_token_and_client_id(
        self,
        access_token_and_client_id: Option<(&Secret, &Secret)>,
    ) -> Self;
}

impl RequestBuilderExt for RequestBuilder {
    fn encode<T>(self, req: &T) -> Self
    where
        T: Request,
    {
        T::Encoding::encode(self, req)
    }

    fn access_token_and_client_id(
        self,
        access_token_and_client_id: Option<(&Secret, &Secret)>,
    ) -> Self {
        if let Some((access_token, client_id)) = access_token_and_client_id {
            self.header(header::AUTHORIZATION, access_token.bearer())
                .header("Client-Id", client_id)
        } else {
            self
        }
    }
}

pub trait Request: Serialize {
    type Encoding: Encoding;
    type Response: DeserializeOwned;

    fn url(&self) -> impl IntoUrl;
}

pub trait Encoding {
    const METHOD: Method;

    fn encode(builder: RequestBuilder, req: &impl Serialize) -> RequestBuilder;
}

pub enum UrlParamEncoding {}

impl Encoding for UrlParamEncoding {
    const METHOD: Method = Method::GET;

    fn encode(builder: RequestBuilder, req: &impl Serialize) -> RequestBuilder {
        builder.query(req)
    }
}

pub enum FormEncoding {}

impl Encoding for FormEncoding {
    const METHOD: Method = Method::POST;

    fn encode(builder: RequestBuilder, req: &impl Serialize) -> RequestBuilder {
        builder.form(req)
    }
}

pub enum JsonEncoding {}

impl Encoding for JsonEncoding {
    const METHOD: Method = Method::POST;

    fn encode(builder: RequestBuilder, req: &impl Serialize) -> RequestBuilder {
        builder.json(req)
    }
}
