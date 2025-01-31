use serde::{Deserialize, Serialize};

use crate::client::{JsonEncoding, NoContent, Request, UrlParamEncoding};

#[derive(Debug, Serialize)]
pub struct ChatColorsRequest {
    /// The ID of the user whose username color you want to get. To specify more than one user, include the user_id parameter for each user to get. For example, &user_id=1234&user_id=5678. The maximum number of IDs that you may specify is 100.
    ///
    /// The API ignores duplicate IDs and IDs that weren’t found.
    user_id: String,
}

impl ChatColorsRequest {
    pub fn id(id: String) -> Self {
        Self { user_id: id }
    }
}

impl Request for ChatColorsRequest {
    type Encoding = UrlParamEncoding;
    type Response = ChatColorsResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/chat/color")
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatColorsResponse {
    /// The list of users and the color code they use for their name.
    data: Vec<ChatColor>,
}

impl ChatColorsResponse {
    pub fn into_chat_color(mut self) -> Option<ChatColor> {
        if self.data.len() > 1 {
            unreachable!("mulitple chat colors returned");
        }
        self.data.pop()
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatColor {
    /// An ID that uniquely identifies the user.
    pub user_id: String,

    /// The user’s login name.
    pub user_login: String,

    /// The user’s display name.
    pub user_name: String,

    /// The Hex color code that the user uses in chat for their name. If the user hasn’t specified a color in their settings, the string is empty.
    pub color: String,
}

#[derive(Debug, Serialize)]
pub struct SendChatMessageRequest {
    /// The ID of the broadcaster whose chat room the message will be sent to.
    pub broadcaster_id: String,

    /// The ID of the user sending the message. This ID must match the user ID in the user access token.
    pub sender_id: String,

    /// The message to send. The message is limited to a maximum of 500 characters. Chat messages can also include emoticons. To include emoticons, use the name of the emote. The names are case sensitive. Don’t include colons around the name (e.g., :bleedPurple:). If Twitch recognizes the name, Twitch converts the name to the emote before writing the chat message to the chat room
    pub message: String,

    /// The ID of the chat message being replied to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_parent_message_id: Option<String>,
}

impl Request for SendChatMessageRequest {
    type Encoding = JsonEncoding;
    type Response = SendChatMessagesResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/chat/messages")
    }
}

#[derive(Debug, Deserialize)]
pub struct SendChatMessagesResponse {
    data: Vec<SentChatMessage>,
}

impl SendChatMessagesResponse {
    pub fn into_chat_message(mut self) -> Option<SentChatMessage> {
        if self.data.len() > 1 {
            unreachable!("mulitple chat messages returned");
        }
        self.data.pop()
    }
}

#[derive(Debug, Deserialize)]
pub struct SentChatMessage {
    /// The message id for the message that was sent.
    pub message_id: String,

    /// If the message passed all checks and was sent.
    pub is_sent: bool,

    /// The reason the message was dropped, if any.
    #[serde(default)]
    pub drop_reason: Option<SentChatMessageDropReason>,
}

#[derive(Debug, Deserialize)]
pub struct SentChatMessageDropReason {
    /// Code for why the message was dropped.
    pub code: String,

    /// Message for why the message was dropped.
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SendChatAnnouncementRequest {
    /// The ID of the broadcaster that owns the chat room to send the announcement to.
    #[serde(skip)]
    pub broadcaster_id: String,

    /// The ID of a user who has permission to moderate the broadcaster’s chat room, or the broadcaster’s ID if they’re sending the announcement. This ID must match the user ID in the user access token.
    #[serde(skip)]
    pub moderator_id: String,

    /// The announcement to make in the broadcaster’s chat room. Announcements are limited to a maximum of 500 characters; announcements longer than 500 characters are truncated.
    pub message: String,

    /// The color used to highlight the announcement. Possible case-sensitive values are:
    ///
    /// If color is set to primary or is not set, the channel’s accent color is used to highlight the announcement (see Profile Accent Color under profile settings, Channel and Videos, and Brand).
    pub color: ChatAnnouncementColor,
}

impl Request for SendChatAnnouncementRequest {
    type Encoding = JsonEncoding;
    type Response = NoContent;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/chat/announcements")
    }

    fn modify_request(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.query(&[
            ("broadcaster_id", &self.broadcaster_id),
            ("moderator_id", &self.moderator_id),
        ])
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum ChatAnnouncementColor {
    #[serde(rename = "blue", alias = "BLUE")]
    Blue,

    #[serde(rename = "green", alias = "GREEN")]
    Green,

    #[serde(rename = "orange", alias = "ORANGE")]
    Orange,

    #[serde(rename = "purple", alias = "PURPLE")]
    Purple,

    #[default]
    #[serde(rename = "primary", alias = "PRIMARY")]
    Primary,
}
