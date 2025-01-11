use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    client::{Request, UrlParamEncoding},
    pagination::Pagination,
    secret::Secret,
};

#[derive(Debug, Serialize)]
pub struct ChannelFollowersRequest {
    /// A user’s ID. Use this parameter to see whether the user follows this broadcaster. If specified, the response contains this user if they follow the broadcaster. If not specified, the response contains all users that follow the broadcaster.
    ///
    /// Using this parameter requires both a user access token with the moderator:read:followers scope and the user ID in the access token match the broadcaster_id or be the user ID for a moderator of the specified broadcaster.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// The broadcaster’s ID. Returns the list of users that follow this broadcaster.
    pub broadcaster_id: String,

    /// The maximum number of items to return per page in the response. The minimum page size is 1 item per page and the maximum is 100. The default is 20.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<usize>,

    /// The cursor used to get the next page of results. The Pagination object in the response contains the cursor’s value. Read more.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Secret>,
}

impl ChannelFollowersRequest {
    pub fn total_only(broadcaster_id: String) -> Self {
        Self {
            user_id: Some("-".into()),
            broadcaster_id,
            first: Some(1),
            after: None,
        }
    }
}

impl Request for ChannelFollowersRequest {
    type Encoding = UrlParamEncoding;
    type Response = ChannelFollowersResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/channels/followers")
    }
}

#[derive(Debug, Deserialize)]
pub struct ChannelFollowersResponse {
    /// The list of users that follow the specified broadcaster. The list is in descending order by followed_at (with the most recent follower first). The list is empty if nobody follows the broadcaster, the specified user_id isn’t in the follower list, the user access token is missing the moderator:read:followers scope, or the user isn’t the broadcaster or moderator for the channel.
    pub data: Vec<ChannelFollower>,

    /// Contains the information used to page through the list of results. The object is empty if there are no more pages left to page through. Read more.
    pub pagination: Pagination,

    /// The total number of users that follow this broadcaster. As someone pages through the list, the number of users may change as users follow or unfollow the broadcaster.
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ChannelFollower {
    /// The UTC timestamp when the user started following the broadcaster.
    pub followed_at: DateTime<Utc>,

    /// An ID that uniquely identifies the user that’s following the broadcaster.
    pub user_id: String,

    /// The user’s login name.
    pub user_login: String,

    /// The user’s display name.
    pub user_name: String,
}
