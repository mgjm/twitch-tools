use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::Subscription;

#[derive(Debug, Deserialize)]
pub struct Follow {
    /// The user ID for the user now following the specified channel.
    pub user_id: String,

    /// The user login for the user now following the specified channel.
    pub user_login: String,

    /// The user display name for the user now following the specified channel.
    pub user_name: String,

    /// The requested broadcaster ID.
    pub broadcaster_user_id: String,

    /// The requested broadcaster login.
    pub broadcaster_user_login: String,

    /// The requested broadcaster display name.
    pub broadcaster_user_name: String,

    /// RFC3339 timestamp of when the follow occurred.
    pub followed_at: DateTime<Utc>,
}

impl Subscription for Follow {
    const TYPE: &'static str = "channel.follow";
    const VERSION: &'static str = "2";

    type Condition = FollowCondition;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FollowCondition {
    /// The broadcaster user ID for the channel you want to get follow notifications for.
    pub broadcaster_user_id: String,

    /// The ID of the moderator of the channel you want to get follow notifications for. If you have authorization from the broadcaster rather than a moderator, specify the broadcaster’s user ID here.
    pub moderator_user_id: String,
}
