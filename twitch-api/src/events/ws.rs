use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message as WsMessage};

use crate::secret::Secret;

use super::{subscription::SubscriptionStatus, types::Subscription};

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

        let (_, message) = Self::next_message(&mut stream)
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

    pub async fn next(&mut self) -> Result<Option<(DateTime<Utc>, NotificationMessage)>> {
        while let Some((timestamp, message)) = Self::next_message(&mut self.stream).await? {
            match message {
                Message::SessionWelcome(message) => {
                    anyhow::bail!("unexpected welcome message: {message:?}")
                }
                Message::SessionKeepalive(_message) => {
                    // eprintln!("session keepalive message");
                }
                Message::Notification(message) => {
                    // eprintln!("{message:#?}");
                    return Ok(Some((timestamp, message)));
                }
            }
        }

        eprintln!("end of web socket stream: {:#?}", self.session_info);

        Ok(None)
    }

    async fn next_message(stream: &mut WsStream) -> Result<Option<(DateTime<Utc>, Message)>> {
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
                    // eprintln!("received message: {:#?}", message.metadata);
                    let (timestamp, message) = Message::from_message(message)?;
                    // eprintln!("{message:#?}");
                    return Ok(Some((timestamp, message)));
                }
                WsMessage::Binary(data) => {
                    anyhow::bail!("received binary websocket message: {} bytes", data.len());
                }
                WsMessage::Ping(data) => {
                    if !data.is_empty() {
                        eprintln!("received ping message: {data:?}");
                    }
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
    pub message_timestamp: DateTime<Utc>,

    /// The type of event sent in the message.
    #[serde(default)]
    pub subscription_type: Option<String>,

    /// The version number of the subscription type’s definition. This is the same value specified in the subscription request.
    #[serde(default)]
    pub subscription_version: Option<String>,
}

#[derive(Debug)]
pub enum Message {
    SessionWelcome(SessionWelcomeMessage),
    SessionKeepalive(SessionKeepaliveMessage),
    Notification(NotificationMessage),
}

impl Message {
    fn from_message(message: WebSocketMessage) -> Result<(DateTime<Utc>, Self)> {
        Ok((
            message.metadata.message_timestamp,
            match message.metadata.message_type.as_str() {
                "session_welcome" => Self::SessionWelcome(message.payload()?),
                "session_keepalive" => Self::SessionKeepalive(message.payload()?),
                "notification" => Self::Notification(message.payload()?),
                message_type => anyhow::bail!("unknown message type: {message_type:?}"),
            },
        ))
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
    pub id: Secret,

    /// The connection’s status.
    pub status: String,

    /// The maximum number of seconds that you should expect silence before receiving a keepalive message. For a welcome message, this is the number of seconds that you have to subscribe to an event after receiving the welcome message. If you don’t subscribe to an event within this window, the socket is disconnected.
    pub keepalive_timeout_seconds: u32,

    /// The URL to reconnect to if you get a Reconnect message.
    pub reconnect_url: Option<Secret>,

    /// The UTC date and time that the connection was created.
    pub connected_at: DateTime<Utc>,

    /// Undocumented by Twitch API reference, but returned
    pub recovery_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionKeepaliveMessage {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationMessage {
    /// An object that contains information about your subscription.
    subscription: SubscriptionInfo,

    /// The event’s data. For information about the event’s data, see the subscription type’s description in Subscription Types.
    event: Value,
}

impl NotificationMessage {
    pub fn event<T>(&self) -> Result<Option<T>>
    where
        T: Subscription,
    {
        parse_event(
            &self.subscription.type_,
            &self.subscription.version,
            &self.event,
        )
    }

    pub fn into_event(self) -> NotificationMessageEvent {
        NotificationMessageEvent {
            type_: self.subscription.type_,
            version: self.subscription.version,
            event: self.event,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationMessageEvent {
    type_: String,
    version: String,
    event: Value,
}

impl NotificationMessageEvent {
    pub fn parse<T>(&self) -> Result<Option<T>>
    where
        T: Subscription,
    {
        parse_event(&self.type_, &self.version, &self.event)
    }
}

pub fn parse_event<T>(type_: &str, version: &str, event: &Value) -> Result<Option<T>>
where
    T: Subscription,
{
    if type_ != T::TYPE {
        return Ok(None);
    };
    anyhow::ensure!(
        version == T::VERSION,
        "subscription version does not match: expected {:?}, got {version:?}",
        T::VERSION,
    );

    serde_json::from_value(event.clone())
        .map(Some)
        .with_context(|| format!("parse notification event: {type_:?} {version:?}"))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionInfo {
    /// An ID that uniquely identifies this subscription.
    pub id: Secret,

    /// The subscription’s status, which is set to enabled.
    pub status: SubscriptionStatus,

    /// The type of event sent in the message. See the event field.
    #[serde(rename = "type")]
    pub type_: String,

    /// The version number of the subscription type’s definition.
    pub version: String,

    /// The event’s cost. See Subscription limits.
    pub cost: u32,

    /// The conditions under which the event fires. For example, if you requested notifications when a broadcaster gets a new follower, this object contains the broadcaster’s ID. For information about the condition’s data, see the subscription type’s description in Subscription types.
    pub condition: Value,

    /// An object that contains information about the transport used for notifications.
    pub transport: TransportInfo,

    /// The UTC date and time that the subscription was created.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportInfo {
    /// The transport method, which is set to websocket.
    pub method: String,

    /// An ID that uniquely identifies the WebSocket connection.
    pub session_id: Secret,
}
