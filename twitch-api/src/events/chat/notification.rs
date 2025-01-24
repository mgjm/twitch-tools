use serde::{Deserialize, Serialize};

use crate::{chat::ChatAnnouncementColor, events::types::Subscription};

use super::{ChatMessageBadge, ChatMessageMessage};

#[derive(Debug, Deserialize)]
pub struct ChatNotification {
    /// The broadcaster user ID.
    pub broadcaster_user_id: String,

    /// The broadcaster display name.
    pub broadcaster_user_name: String,

    /// The broadcaster login.
    pub broadcaster_user_login: String,

    /// The user ID of the user that sent the message.
    pub chatter_user_id: String,

    /// The user login of the user that sent the message.
    pub chatter_user_name: String,

    /// Whether or not the chatter is anonymous.
    pub chatter_is_anonymous: bool,

    /// The color of the user’s name in the chat room.
    pub color: String,

    /// The color of the user’s name in the chat room.
    pub badges: Vec<ChatMessageBadge>,

    /// The message Twitch shows in the chat room for this notice.
    pub system_message: String,

    /// A UUID that identifies the message.
    pub message_id: String,

    /// The structured chat message.
    pub message: ChatMessageMessage,

    /// The type of notice. Possible values are:
    #[serde(flatten)]
    pub notice_type: ChatNotificationType,

    // --------------------------------------------------------------------------------
    /// Optional. The broadcaster user ID of the channel the message was sent from. Is null when the message notification happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_broadcaster_user_id: Option<String>,

    /// Optional. The user name of the broadcaster of the channel the message was sent from. Is null when the message notification happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_broadcaster_user_name: Option<String>,

    /// Optional. The login of the broadcaster of the channel the message was sent from. Is null when the message notification happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_broadcaster_user_login: Option<String>,

    /// Optional. The UUID that identifies the source message from the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_message_id: Option<String>,

    /// Optional. The list of chat badges for the chatter in the channel the message was sent from. Is null when the message happens in the same channel as the broadcaster. Is not null when in a shared chat session, and the action happens in the channel of a participant other than the broadcaster.
    #[serde(default)]
    pub source_badges: Option<Vec<ChatMessageBadge>>,
}

impl Subscription for ChatNotification {
    const TYPE: &'static str = "channel.chat.notification";
    const VERSION: &'static str = "1";

    type Condition = ChatNotificationCondition;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatNotificationCondition {
    /// User ID of the channel to receive chat notification events for.
    pub broadcaster_user_id: String,

    /// The User ID to read chat as.
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "notice_type")]
pub enum ChatNotificationType {
    #[serde(rename = "sub")]
    Sub {
        /// Information about the sub event. Null if notice_type is not sub.
        sub: ChatNotificationSub,
    },

    #[serde(rename = "resub")]
    Resub {
        /// Information about the resub event. Null if notice_type is not resub.
        resub: ChatNotificationResub,
    },

    #[serde(rename = "sub_gift")]
    SubGift {
        /// Information about the gift sub event. Null if notice_type is not sub_gift.
        sub_gift: ChatNotificationSubGift,
    },

    #[serde(rename = "community_sub_gift")]
    CommunitySubGift {
        /// Information about the community gift sub event. Null if notice_type is not community_sub_gift.
        community_sub_gift: ChatNotificationCommunitySubGift,
    },

    #[serde(rename = "gift_paid_upgrade")]
    GiftPaidUpgrade {
        /// Information about the community gift paid upgrade event. Null if notice_type is not gift_paid_upgrade.
        gift_paid_upgrade: ChatNotificationGiftPaidUpgrade,
    },

    #[serde(rename = "prime_paid_upgrade")]
    PrimePaidUpgrade {
        /// Information about the Prime gift paid upgrade event. Null if notice_type is not prime_paid_upgrade
        prime_paid_upgrade: ChatNotificationPrimePaidUpgrade,
    },

    #[serde(rename = "raid")]
    Raid {
        /// Information about the raid event. Null if notice_type is not raid
        raid: ChatNotificationRaid,
    },

    #[serde(rename = "unraid")]
    Unraid {
        /// Returns an empty payload if notice_type is not unraid, otherwise returns null.
        unraid: ChatNotificationUnraid,
    },

    #[serde(rename = "pay_it_forward")]
    PayItForward {
        /// Information about the pay it forward event. Null if notice_type is not pay_it_forward
        pay_it_forward: ChatNotificationPayItForward,
    },

    #[serde(rename = "announcement")]
    Announcement {
        /// Information about the announcement event. Null if notice_type is not announcement
        announcement: ChatNotificationAnnouncement,
    },

    #[serde(rename = "bits_badge_tier")]
    BitsBadgeTier {
        /// Information about the bits badge tier event. Null if notice_type is not bits_badge_tier
        bits_badge_tier: ChatNotificationBitsBadgeTier,
    },

    #[serde(rename = "charity_donation")]
    CharityDonation {
        /// Information about the announcement event. Null if notice_type is not charity_donation
        charity_donation: ChatNotificationCharityDonation,
    },

    #[serde(rename = "shared_chat_sub")]
    SharedChatSub {
        /// Information about the shared_chat_sub event. Is null if notice_type is not shared_chat_sub.
        ///
        /// This field has the same information as the sub field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_sub: ChatNotificationSub,
    },

    #[serde(rename = "shared_chat_resub")]
    SharedChatResub {
        /// Information about the shared_chat_resub event. Is null if notice_type is not shared_chat_resub.
        ///
        /// This field has the same information as the resub field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_resub: ChatNotificationResub,
    },

    #[serde(rename = "shared_chat_sub_gift")]
    SharedChatSubGift {
        /// Information about the shared_chat_sub_gift event. Is null if notice_type is not shared_chat_sub_gift.
        ///
        /// This field has the same information as the chat_sub_gift field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_sub_gift: ChatNotificationSubGift,
    },

    #[serde(rename = "shared_chat_community_sub_gift")]
    SharedChatCommunitySubGift {
        /// Information about the shared_chat_community_sub_gift event. Is null if notice_type is not shared_chat_community_sub_gift.
        ///
        /// This field has the same information as the community_sub_gift field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_community_sub_gift: ChatNotificationCommunitySubGift,
    },

    #[serde(rename = "shared_chat_gift_paid_upgrade")]
    SharedChatGiftPaidUpgrade {
        /// Information about the shared_chat_gift_paid_upgrade event. Is null if notice_type is not shared_chat_gift_paid_upgrade.
        ///
        /// This field has the same information as the gift_paid_upgrade field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_gift_paid_upgrade: ChatNotificationGiftPaidUpgrade,
    },

    #[serde(rename = "shared_chat_prime_paid_upgrade")]
    SharedChatPrimePaidUpgrade {
        /// Information about the shared_chat_chat_prime_paid_upgrade event. Is null if notice_type is not shared_chat_prime_paid_upgrade.
        ///
        /// This field has the same information as the prime_paid_upgrade field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_prime_paid_upgrade: ChatNotificationPrimePaidUpgrade,
    },

    #[serde(rename = "shared_chat_raid")]
    SharedChatRaid {
        /// Information about the shared_chat_raid event. Is null if notice_type is not shared_chat_raid.
        ///
        /// This field has the same information as the raid field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_raid: ChatNotificationRaid,
    },

    #[serde(rename = "shared_chat_pay_it_forward")]
    SharedChatPayItForward {
        /// Information about the shared_chat_pay_it_forward event. Is null if notice_type is not shared_chat_pay_it_forward.
        ///
        /// This field has the same information as the pay_it_forward field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_pay_it_forward: ChatNotificationPayItForward,
    },

    #[serde(rename = "shared_chat_announcement")]
    SharedChatAnnouncement {
        /// Information about the shared_chat_announcement event. Is null if notice_type is not shared_chat_announcement.
        ///
        /// This field has the same information as the announcement field but for a notice that happened for a channel in a shared chat session other than the broadcaster in the subscription condition.
        shared_chat_announcement: ChatNotificationAnnouncement,
    },
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationSub {
    /// The type of subscription plan being used. Possible values are:
    pub sub_tier: SubTier,

    /// Indicates if the subscription was obtained through Amazon Prime.
    pub is_prime: bool,

    /// The number of months the subscription is for.
    pub duration_months: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationResub {
    /// The total number of months the user has subscribed.
    pub cumulative_months: u32,

    /// The number of months the subscription is for.
    pub duration_months: u32,

    /// The total number of months the user has subscribed.
    pub streak_months: u32,

    /// The type of subscription plan being used. Possible values are:
    pub sub_tier: SubTier,

    /// Optional. The number of consecutive months the user has subscribed.
    pub is_prime: bool,

    /// Whether or not the resub was a result of a gift.
    pub is_gift: bool,

    /// Optional. Whether or not the gift was anonymous.
    pub gifter_is_anonymous: bool,

    /// The user ID of the subscription gifter. Null if anonymous.
    pub gifter_user_id: String,

    /// The user name of the subscription gifter. Null if anonymous.
    pub gifter_user_name: String,

    /// Optional. The user login of the subscription gifter. Null if anonymous.
    #[serde(default)]
    pub gifter_user_login: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationSubGift {
    /// The number of months the subscription is for.
    pub duration_months: u32,

    /// Optional. The amount of gifts the gifter has given in this channel. Null if anonymous.
    #[serde(default)]
    pub cumulative_total: Option<u32>,

    /// The user ID of the subscription gift recipient.
    pub recipient_user_id: String,

    /// The user name of the subscription gift recipient.
    pub recipient_user_name: String,

    /// The user login of the subscription gift recipient.
    pub recipient_user_login: String,

    /// The type of subscription plan being used. Possible values are:
    pub sub_tier: SubTier,

    /// Optional. The ID of the associated community gift. Null if not associated with a community gift.
    #[serde(default)]
    pub community_gift_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationCommunitySubGift {
    /// The ID of the associated community gift.
    pub id: String,

    /// Number of subscriptions being gifted.
    pub total: u32,

    /// The type of subscription plan being used.
    pub sub_tier: SubTier,

    /// Optional. The amount of gifts the gifter has given in this channel. Null if anonymous.
    #[serde(default)]
    pub cumulative_total: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationGiftPaidUpgrade {
    /// Whether the gift was given anonymously.
    pub gifter_is_anonymous: bool,

    /// Optional. The user ID of the user who gifted the subscription. Null if anonymous.
    #[serde(default)]
    pub gifter_user_id: Option<String>,

    /// Optional. The user name of the user who gifted the subscription. Null if anonymous.
    #[serde(default)]
    pub gifter_user_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationPrimePaidUpgrade {
    /// The type of subscription plan being used. Possible values are:
    pub sub_tier: SubTier,
}

#[derive(Debug, Deserialize)]
pub enum SubTier {
    /// First level of paid or Prime subscription.
    #[serde(rename = "1000")]
    FirstLevel,

    /// Second level of paid subscription.
    #[serde(rename = "2000")]
    SecondLevel,

    /// Third level of paid subscription.
    #[serde(rename = "3000")]
    ThirdLevel,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationRaid {
    /// The user ID of the broadcaster raiding this channel.
    pub user_id: String,

    /// The user name of the broadcaster raiding this channel.
    pub user_name: String,

    /// The login name of the broadcaster raiding this channel.
    pub user_login: String,

    /// The number of viewers raiding this channel from the broadcaster’s channel.
    pub viewer_count: u32,

    /// Profile image URL of the broadcaster raiding this channel.
    pub profile_image_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationUnraid {}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationPayItForward {
    /// Whether the gift was given anonymously.
    pub gifter_is_anonymous: bool,

    /// The user ID of the user who gifted the subscription. Null if anonymous.
    pub gifter_user_id: String,

    /// Optional. The user name of the user who gifted the subscription. Null if anonymous.
    #[serde(default)]
    pub gifter_user_name: Option<String>,

    /// The user login of the user who gifted the subscription. Null if anonymous.
    pub gifter_user_login: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationAnnouncement {
    /// Color of the announcement.
    pub color: ChatAnnouncementColor,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationBitsBadgeTier {
    /// The tier of the Bits badge the user just earned. For example, 100, 1000, or 10000.
    pub tier: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationCharityDonation {
    /// Name of the charity.
    pub charity_name: String,

    /// An object that contains the amount of money that the user paid.
    pub amount: ChatNotificationCharityDonationAmount,
}

#[derive(Debug, Deserialize)]
pub struct ChatNotificationCharityDonationAmount {
    /// The monetary amount. The amount is specified in the currency’s minor unit. For example, the minor units for USD is cents, so if the amount is $5.50 USD, value is set to 550.
    pub value: u32,

    /// The number of decimal places used by the currency. For example, USD uses two decimal places.
    pub decimal_place: u32,

    /// The ISO-4217 three-letter currency code that identifies the type of currency in value.
    pub currency: String,
}
