use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message as WsMessage};

use crate::secret::Secret;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WebSocket {
    stream: WsStream,
    session_info: SessionInfo,
}

impl WebSocket {
    pub async fn connect() -> Result<Self> {
        let (mut stream, _response) =
            tokio_tungstenite::connect_async("wss://eventsub.wss.twitch.tv/ws")
                .await
                .context("connect to ws server")?;

        let message = Self::next_message(&mut stream)
            .await?
            .context("missing welcome message")?;
        let Message::SessionWelcome(message) = message else {
            anyhow::bail!("expected welcome message, got: {message:?}");
        };

        Ok(Self {
            stream,
            session_info: message.session,
        })
    }

    pub fn session_id(&self) -> &Secret {
        &self.session_info.id
    }

    pub async fn next(&mut self) -> Result<Option<NotificationMessage>> {
        while let Some(message) = Self::next_message(&mut self.stream).await? {
            match message {
                Message::SessionWelcome(message) => {
                    anyhow::bail!("unexpected welcome message: {message:?}")
                }
                Message::SessionKeepalive(_message) => {
                    eprintln!("session keepalive message");
                }
                Message::Notification(message) => return Ok(Some(message)),
            }
        }

        eprintln!("end of web socket stream: {:#?}", self.session_info);

        Ok(None)
    }

    async fn next_message(stream: &mut WsStream) -> Result<Option<Message>> {
        while let Some(message) = stream
            .next()
            .await
            .transpose()
            .context("receive next websocket message")?
        {
            match message {
                WsMessage::Text(data) => {
                    let message: WebSocketMessage =
                        serde_json::from_str(data.as_str()).context("parse websocket message")?;
                    eprintln!("received message: {:#?}", message.metadata);
                    let message = Message::from_message(message)?;
                    eprintln!("{message:#?}");
                    return Ok(Some(message));
                }
                WsMessage::Binary(data) => {
                    anyhow::bail!("received binary websocket message: {} bytes", data.len());
                }
                WsMessage::Ping(data) => {
                    eprintln!("received ping message: {data:?}");
                    stream
                        .send(WsMessage::Pong(data))
                        .await
                        .context("send pong response")?;
                }
                WsMessage::Pong(data) => eprintln!("received pong message: {data:?}"),
                WsMessage::Close(None) => {
                    eprintln!("close without close frame");
                    break;
                }
                WsMessage::Close(Some(close_frame)) => {
                    eprintln!(
                        "close with close frame: {} {:?}",
                        close_frame.code,
                        close_frame.reason.as_str(),
                    );
                    break;
                }
                WsMessage::Frame(_) => unreachable!("raw websocket frame"),
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebSocketMessage {
    /// An object that identifies the message.
    pub metadata: WebSocketMetadata,

    /// An object that contains the message.
    payload: Value,
}

impl WebSocketMessage {
    fn payload<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(self.payload).context("parse message payload")
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebSocketMetadata {
    /// An ID that uniquely identifies the message. Twitch sends messages at least once, but if Twitch is unsure of whether you received a notification, it’ll resend the message. This means you may receive a notification twice. If Twitch resends the message, the message ID will be the same.
    pub message_id: String,

    /// The type of message, which is set to session_welcome.
    pub message_type: String,

    /// The UTC date and time that the message was sent.
    pub message_timestamp: String,
}

#[derive(Debug)]
pub enum Message {
    SessionWelcome(SessionWelcomeMessage),
    SessionKeepalive(SessionKeepaliveMessage),
    Notification(NotificationMessage),
}

impl Message {
    fn from_message(message: WebSocketMessage) -> Result<Self> {
        Ok(match message.metadata.message_type.as_str() {
            "session_welcome" => Self::SessionWelcome(message.payload()?),
            "session_keepalive" => Self::SessionKeepalive(message.payload()?),
            message_type => anyhow::bail!("unknown message type: {message_type:?}"),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionWelcomeMessage {
    /// An object that contains information about the connection.
    session: SessionInfo,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionInfo {
    /// An ID that uniquely identifies this WebSocket connection. Use this ID to set the session_id field in all subscription requests.
    id: Secret,

    /// The connection’s status.
    status: String,

    /// The maximum number of seconds that you should expect silence before receiving a keepalive message. For a welcome message, this is the number of seconds that you have to subscribe to an event after receiving the welcome message. If you don’t subscribe to an event within this window, the socket is disconnected.
    keepalive_timeout_seconds: u32,

    /// The URL to reconnect to if you get a Reconnect message.
    reconnect_url: Option<Secret>,

    /// The UTC date and time that the connection was created.
    connected_at: String,

    /// Undocumented by Twitch API reference, but returned
    recovery_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionKeepaliveMessage {}

#[derive(Debug)]
pub struct NotificationMessage {}
