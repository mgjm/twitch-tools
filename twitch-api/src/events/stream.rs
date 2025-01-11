use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::Subscription;

#[derive(Debug, Deserialize)]
pub struct StreamOnline {
    /// The id of the stream.
    pub id: String,

    /// The broadcaster’s user id.
    pub broadcaster_user_id: String,

    /// The broadcaster’s user login.
    pub broadcaster_user_login: String,

    /// The broadcaster’s user display name.
    pub broadcaster_user_name: String,

    /// The stream type. Valid values are: live, playlist, watch_party, premiere, rerun.
    #[serde(rename = "type")]
    pub type_: StreamType,

    /// The timestamp at which the stream went online at.
    pub started_at: DateTime<Utc>,
}

impl Subscription for StreamOnline {
    const TYPE: &'static str = "stream.online";
    const VERSION: &'static str = "1";

    type Condition = StreamOnlineCondition;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamOnlineCondition {
    /// The broadcaster user ID you want to get stream online notifications for.
    pub broadcaster_user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamOffline {
    /// The broadcaster’s user id.
    pub broadcaster_user_id: String,

    /// The broadcaster’s user login.
    pub broadcaster_user_login: String,

    /// The broadcaster’s user display name.
    pub broadcaster_user_name: String,
}

impl Subscription for StreamOffline {
    const TYPE: &'static str = "stream.offline";
    const VERSION: &'static str = "1";

    type Condition = StreamOfflineCondition;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamOfflineCondition {
    /// The broadcaster user ID you want to get stream offline notifications for.
    pub broadcaster_user_id: String,
}

#[derive(Debug, Deserialize)]
pub enum StreamType {
    #[serde(rename = "live")]
    Live,

    #[serde(rename = "playlist")]
    Playlist,

    #[serde(rename = "watch_party")]
    WatchParty,

    #[serde(rename = "premiere")]
    Premiere,

    #[serde(rename = "rerun")]
    Rerun,
}
