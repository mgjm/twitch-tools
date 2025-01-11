use serde::{Deserialize, Serialize};

use super::types::Subscription;

#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    /// The broadcaster user ID.
    pub broadcaster_user_id: String,

    /// The broadcaster display name.
    pub broadcaster_user_name: String,

    /// The broadcaster login.
    pub broadcaster_user_login: String,

    /// The user ID of the user that sent the message.
    pub chatter_user_id: String,

    /// The user name of the user that sent the message.
    pub chatter_user_name: String,

    /// The user login of the user that sent the message.
    pub chatter_user_login: String,

    /// A UUID that identifies the message.
    pub message_id: String,

    /// The structured chat message.
    pub message: ChatMessageMessage,

    /// The type of message. Possible values:
    ///
    pub message_type: ChatMessageType,

    /// List of chat badges.
    pub badges: Vec<ChatMessageBadge>,

    /// Optional. Metadata if this message is a cheer.
    #[serde(default)]
    pub cheer: Option<ChatMessageCheer>,

    /// The color of the user’s name in the chat room. This is a hexadecimal RGB color code in the form, #&lt;RGB&gt;. This tag may be empty if it is never set.
    pub color: String,

    /// Optional. Metadata if this message is a reply.
    #[serde(default)]
    pub reply: Option<ChatMessageReply>,

    /// Optional. The ID of a channel points custom reward that was redeemed.
    #[serde(default)]
    pub channel_points_custom_reward_id: Option<String>,

    /// Optional. The broadcaster user ID of the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_broadcaster_user_id: Option<String>,

    /// Optional. The user name of the broadcaster of the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_broadcaster_user_name: Option<String>,

    /// Optional. The login of the broadcaster of the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_broadcaster_user_login: Option<String>,

    /// Optional. The UUID that identifies the source message from the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_message_id: Option<String>,

    /// Optional. The list of chat badges for the chatter in the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_badges: Option<ChatMessageSourceBadges>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageMessage {
    /// The chat message in plain text.
    pub text: String,

    /// Ordered list of chat message fragments.
    pub fragments: Vec<ChatMessageFragment>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ChatMessageFragment {
    #[serde(rename = "text")]
    Text {
        /// Message text in fragment.
        text: String,
    },

    #[serde(rename = "cheermote")]
    Cheermote {
        /// Message text in fragment.
        text: String,

        /// Metadata pertaining to the cheermote.
        cheermote: ChatMessageCheermote,
    },

    #[serde(rename = "emote")]
    Emote {
        /// Message text in fragment.
        text: String,

        /// Metadata pertaining to the emote.
        emote: ChatMessageEmote,
    },

    #[serde(rename = "mention")]
    Mention {
        /// Message text in fragment.
        text: String,

        /// Metadata pertaining to the mention.
        mention: ChatMessageMention,
    },
}

impl ChatMessageFragment {
    pub fn text(&self) -> &str {
        let (Self::Text { text }
        | Self::Cheermote { text, .. }
        | Self::Emote { text, .. }
        | Self::Mention { text, .. }) = self;
        text
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageCheermote {
    /// The name portion of the Cheermote string that you use in chat to cheer Bits. The full Cheermote string is the concatenation of {prefix} + {number of Bits}. For example, if the prefix is “Cheer” and you want to cheer 100 Bits, the full Cheermote string is Cheer100. When the Cheermote string is entered in chat, Twitch converts it to the image associated with the Bits tier that was cheered.
    pub prefix: String,

    /// The amount of bits cheered.
    pub bits: u32,

    /// The tier level of the cheermote.
    pub tier: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageEmote {
    /// An ID that uniquely identifies this emote.
    pub id: String,

    /// An ID that identifies the emote set that the emote belongs to.
    pub emote_set_id: String,

    /// The ID of the broadcaster who owns the emote.
    pub owner_id: String,

    /// The formats that the emote is available in. For example, if the emote is available only as a static PNG, the array contains only static. But if the emote is available as a static PNG and an animated GIF, the array contains static and animated. The possible formats are:
    ///
    pub format: Vec<ChatMessageEmoteFormat>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageMention {
    /// The user ID of the mentioned user.
    pub user_id: String,

    /// The user name of the mentioned user.
    pub user_name: String,

    /// The user login of the mentioned user.
    pub user_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageBadge {
    /// An ID that identifies this set of chat badges. For example, Bits or Subscriber.
    pub set_id: String,

    /// An ID that identifies this version of the badge. The ID can be any value. For example, for Bits, the ID is the Bits tier level, but for World of Warcraft, it could be Alliance or Horde.
    pub id: String,

    /// Contains metadata related to the chat badges in the badges tag. Currently, this tag contains metadata only for subscriber badges, to indicate the number of months the user has been a subscriber.
    pub info: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageCheer {
    /// The amount of Bits the user cheered.
    pub bits: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageReply {
    /// An ID that uniquely identifies the parent message that this message is replying to.
    pub parent_message_id: String,

    /// The message body of the parent message.
    pub parent_message_body: String,

    /// User ID of the sender of the parent message.
    pub parent_user_id: String,

    /// User name of the sender of the parent message.
    pub parent_user_name: String,

    /// User login of the sender of the parent message.
    pub parent_user_login: String,

    /// An ID that identifies the parent message of the reply thread.
    pub thread_message_id: String,

    /// User ID of the sender of the thread’s parent message.
    pub thread_user_id: String,

    /// User name of the sender of the thread’s parent message.
    pub thread_user_name: String,

    /// User login of the sender of the thread’s parent message.
    pub thread_user_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageSourceBadges {
    /// The ID that identifies this set of chat badges. For example, Bits or Subscriber.
    pub set_id: String,

    /// The ID that identifies this version of the badge. The ID can be any value. For example, for Bits, the ID is the Bits tier level, but for World of Warcraft, it could be Alliance or Horde.
    pub id: String,

    /// Contains metadata related to the chat badges in the badges tag. Currently, this tag contains metadata only for subscriber badges, to indicate the number of months the user has been a subscriber.
    pub info: String,
}

impl Subscription for ChatMessage {
    const TYPE: &'static str = "channel.chat.message";
    const VERSION: &'static str = "1";

    type Condition = ChatMessageCondition;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageCondition {
    /// The User ID of the channel to receive chat message events for.
    pub broadcaster_user_id: String,

    /// The User ID to read chat as.
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub enum ChatMessageType {
    #[serde(rename = "text")]
    Text,

    #[serde(rename = "channel_points_highlighted")]
    ChannelPointsHighlighted,

    #[serde(rename = "channel_points_sub_only")]
    ChannelPointsSubOnly,

    #[serde(rename = "user_intro")]
    UserIntro,

    #[serde(rename = "power_ups_message_effect")]
    PowerUpsMessageEffect,

    #[serde(rename = "power_ups_gigantified_emote")]
    PowerUpsGigantifiedEmote,
}

#[derive(Debug, Deserialize)]
pub enum ChatMessageEmoteFormat {
    /// An animated GIF is available for this emote.
    #[serde(rename = "animated")]
    Animated,

    /// A static PNG file is available for this emote.
    #[serde(rename = "static")]
    Static,
}
