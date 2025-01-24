use serde::Deserialize;

pub mod message;
pub mod notification;

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
pub enum ChatMessageEmoteFormat {
    /// An animated GIF is available for this emote.
    #[serde(rename = "animated")]
    Animated,

    /// A static PNG file is available for this emote.
    #[serde(rename = "static")]
    Static,
}
