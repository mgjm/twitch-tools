use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::{DeleteUrlParamEncoding, JsonEncoding, Request, UrlParamEncoding},
    pagination::Pagination,
    secret::Secret,
};

use super::types::Subscription;

#[derive(Debug, Serialize)]
pub struct CreateSubscriptionRequest {
    /// The type of subscription to create. For a list of subscriptions that you can create, see Subscription Types. Set this field to the value in the Name column of the Subscription Types table.
    #[serde(rename = "type")]
    type_: &'static str,

    /// The version number that identifies the definition of the subscription type that you want the response to use.
    version: &'static str,

    /// A JSON object that contains the parameter values that are specific to the specified subscription type. For the object’s required and optional fields, see the subscription type’s documentation.
    condition: Value,

    /// The transport details that you want Twitch to use when sending you notifications.
    transport: TransportRequest,
}

impl CreateSubscriptionRequest {
    pub fn new<T>(condition: &T::Condition, transport: TransportRequest) -> Result<Self>
    where
        T: Subscription,
    {
        Ok(Self {
            type_: T::TYPE,
            version: T::VERSION,
            condition: serde_json::to_value(condition).context("convert subscription condition")?,
            transport,
        })
    }
}

impl Request for CreateSubscriptionRequest {
    type Encoding = JsonEncoding;
    type Response = CreateSubscriptionResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/eventsub/subscriptions")
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
pub enum TransportRequest {
    #[serde(rename = "webhook")]
    WebHook {
        /// The callback URL where the notifications are sent. The URL must use the HTTPS protocol and port 443. See Processing an event. Specify this field only if method is set to webhook.
        ///
        /// NOTE: Redirects are not followed.
        callback: Secret,

        /// The secret used to verify the signature. The secret must be an ASCII string that’s a minimum of 10 characters long and a maximum of 100 characters long. For information about how the secret is used, see Verifying the event message. Specify this field only if method is set to webhook.
        secret: Secret,
    },

    #[serde(rename = "websocket")]
    WebSocket {
        /// An ID that identifies the WebSocket to send notifications to. When you connect to EventSub using WebSockets, the server returns the ID in the Welcome message. Specify this field only if method is set to websocket.
        session_id: Secret,
    },

    #[serde(rename = "conduit")]
    Conduit {
        /// An ID that identifies the conduit to send notifications to. When you create a conduit, the server returns the conduit ID. Specify this field only if method is set to conduit.
        conduit_id: Secret,
    },
}

#[derive(Debug, Default, Serialize)]
pub struct GetSubscriptionsRequest {
    /// Filter subscriptions by its status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SubscriptionStatus>,

    /// Filter subscriptions by subscription type. For a list of subscription types, see Subscription Types.
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Filter subscriptions by user ID. The response contains subscriptions where this ID matches a user ID that you specified in the Condition object when you created the subscription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// The cursor used to get the next page of results. The pagination object in the response contains the cursor's value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

impl Request for GetSubscriptionsRequest {
    type Encoding = UrlParamEncoding;
    type Response = GetSubscriptionsResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/eventsub/subscriptions")
    }
}
#[derive(Debug, Serialize)]
pub struct DeleteSubscriptionRequest {
    /// The ID of the subscription to delete.
    pub id: Secret,
}

impl Request for DeleteSubscriptionRequest {
    type Encoding = DeleteUrlParamEncoding;
    type Response = ();

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/eventsub/subscriptions")
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionResponse {
    /// A list that contains the single subscription that you created.
    pub data: Vec<SubscriptionInfo>,

    /// The total number of subscriptions you’ve created.
    pub total: u32,

    /// The sum of all of your subscription costs. Learn More
    pub total_cost: u32,

    /// The maximum total cost that you’re allowed to incur for all subscriptions you create.
    pub max_total_cost: u32,
}

#[derive(Debug, Deserialize)]
pub struct GetSubscriptionsResponse {
    /// A list that contains the single subscription that you created.
    pub data: Vec<SubscriptionInfo>,

    /// The total number of subscriptions you’ve created.
    pub total: u32,

    /// The sum of all of your subscription costs. Learn More
    pub total_cost: u32,

    /// The maximum total cost that you’re allowed to incur for all subscriptions you create.
    pub max_total_cost: u32,

    /// An object that contains the cursor used to get the next page of subscriptions. The object is empty if there are no more pages to get. The number of subscriptions returned per page is undertermined.
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
pub struct SubscriptionInfo {
    /// An ID that identifies the subscription.
    pub id: Secret,

    /// The subscription’s status.
    pub status: SubscriptionStatus,

    /// The subscription’s type. See Subscription Types.
    #[serde(rename = "type")]
    pub type_: String,

    /// The version number that identifies this definition of the subscription’s data.
    pub version: String,

    /// The subscription’s parameter values. This is a string-encoded JSON object whose contents are determined by the subscription type.
    pub condition: Value,

    /// The date and time (in RFC3339 format) of when the subscription was created.
    pub created_at: DateTime<Utc>,

    /// The transport details used to send the notifications.
    pub transport: TransportResponse,

    /// The amount that the subscription counts against your limit.
    pub cost: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum TransportResponse {
    #[serde(rename = "webhook")]
    WebHook {
        /// The callback URL where the notifications are sent. Included only if method is set to webhook.
        callback: Secret,
    },

    #[serde(rename = "websocket")]
    WebSocket {
        /// An ID that identifies the WebSocket that notifications are sent to. Included only if method is set to websocket.
        session_id: Secret,

        /// The UTC date and time that the WebSocket connection was established. Included only if method is set to websocket.
        connected_at: DateTime<Utc>,
    },

    #[serde(rename = "conduit")]
    Conduit {
        /// An ID that identifies the conduit to send notifications to. Included only if method is set to conduit.
        conduit_id: Secret,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    /// The subscription is enabled.
    #[serde(rename = "enabled")]
    Enabled,

    /// The subscription is pending verification of the specified callback URL.
    #[serde(rename = "webhook_callback_verification_pending")]
    WebhookCallbackVerificationPending,

    /// The specified callback URL failed verification.
    #[serde(rename = "webhook_callback_verification_failed")]
    WebhookCallbackVerificationFailed,

    /// The notification delivery failure rate was too high.
    #[serde(rename = "notification_failures_exceeded")]
    NotificationFailuresExceeded,

    /// The authorization was revoked for one or more users specified in the Condition object.
    #[serde(rename = "authorization_revoked")]
    AuthorizationRevoked,

    /// The moderator that authorized the subscription is no longer one of the broadcaster's moderators.
    #[serde(rename = "moderator_removed")]
    ModeratorRemoved,

    /// One of the users specified in the Condition object was removed.
    #[serde(rename = "user_removed")]
    UserRemoved,

    /// The user specified in the Condition object was banned from the broadcaster's chat.
    #[serde(rename = "chat_user_banned")]
    ChatUserBanned,

    /// The subscription to subscription type and version is no longer supported.
    #[serde(rename = "version_removed")]
    VersionRemoved,

    /// The subscription to the beta subscription type was removed due to maintenance.
    #[serde(rename = "beta_maintenance")]
    BetaMaintenance,

    /// The client closed the connection.
    #[serde(rename = "websocket_disconnected")]
    WebsocketDisconnected,

    /// The client failed to respond to a ping message.
    #[serde(rename = "websocket_failed_ping_pong")]
    WebsocketFailedPingPong,

    /// The client sent a non-pong message. Clients may only send pong messages (and only in response to a ping message).
    #[serde(rename = "websocket_received_inbound_traffic")]
    WebsocketReceivedInboundTraffic,

    /// The client failed to subscribe to events within the required time.
    #[serde(rename = "websocket_connection_unused")]
    WebsocketConnectionUnused,

    /// The Twitch WebSocket server experienced an unexpected error.
    #[serde(rename = "websocket_internal_error")]
    WebsocketInternalError,

    /// The Twitch WebSocket server timed out writing the message to the client.
    #[serde(rename = "websocket_network_timeout")]
    WebsocketNetworkTimeout,

    /// The Twitch WebSocket server experienced a network error writing the message to the client.
    #[serde(rename = "websocket_network_error")]
    WebsocketNetworkError,

    /// The client failed to reconnect to the Twitch WebSocket server within the required time after a Reconnect Message.
    #[serde(rename = "websocket_failed_to_reconnect")]
    WebsocketFailedToReconnect,
}
