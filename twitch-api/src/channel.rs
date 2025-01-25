use serde::{Deserialize, Serialize};

use crate::client::{Request, UrlParamEncoding};

#[derive(Debug, Serialize)]
pub struct ChannelsRequest {
    /// The ID of the broadcaster whose channel you want to get. To specify more than one ID, include this parameter for each broadcaster you want to get. For example, broadcaster_id=1234&broadcaster_id=5678. You may specify a maximum of 100 IDs. The API ignores duplicate IDs and IDs that are not found.
    broadcaster_id: String,
}

impl ChannelsRequest {
    pub fn id(id: String) -> Self {
        Self { broadcaster_id: id }
    }
}

impl Request for ChannelsRequest {
    type Encoding = UrlParamEncoding;
    type Response = ChannelsResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/channels")
    }
}

#[derive(Debug, Deserialize)]
pub struct ChannelsResponse {
    /// A list that contains information about the specified channels. The list is empty if the specified channels weren’t found.
    pub data: Vec<Channel>,
}

impl ChannelsResponse {
    pub fn into_channel(mut self) -> Option<Channel> {
        if self.data.len() > 1 {
            unreachable!("mulitple channels returned");
        }
        self.data.pop()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Channel {
    /// An ID that uniquely identifies the broadcaster.
    pub broadcaster_id: String,

    /// The broadcaster’s login name.
    pub broadcaster_login: String,

    /// The broadcaster’s display name.
    pub broadcaster_name: String,

    /// The broadcaster’s preferred language. The value is an ISO 639-1 two-letter language code (for example, en for English). The value is set to “other” if the language is not a Twitch supported language.
    pub broadcaster_language: String,

    /// The name of the game that the broadcaster is playing or last played. The value is an empty string if the broadcaster has never played a game.
    pub game_name: String,

    /// An ID that uniquely identifies the game that the broadcaster is playing or last played. The value is an empty string if the broadcaster has never played a game.
    pub game_id: String,

    /// The title of the stream that the broadcaster is currently streaming or last streamed. The value is an empty string if the broadcaster has never streamed.
    pub title: String,

    /// The value of the broadcaster’s stream delay setting, in seconds. This field’s value defaults to zero unless 1) the request specifies a user access token, 2) the ID in the broadcaster_id query parameter matches the user ID in the access token, and 3) the broadcaster has partner status and they set a non-zero stream delay value.
    pub delay: u32,

    /// The tags applied to the channel.
    pub tags: Vec<String>,

    /// The CCLs applied to the channel.
    pub content_classification_labels: Vec<String>,

    /// Boolean flag indicating if the channel has branded content.
    pub is_branded_content: bool,
}
