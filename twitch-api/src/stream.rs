use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    client::{Request, UrlParamEncoding},
    events::stream::StreamType,
    pagination::Pagination,
    secret::Secret,
};

#[derive(Debug, Serialize)]
pub struct StreamsRequest {
    /// A user ID used to filter the list of streams. Returns only the streams of those users that are broadcasting. You may specify a maximum of 100 IDs. To specify multiple IDs, include the user_id parameter for each user. For example, &user_id=1234&user_id=5678.
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,

    /// A user login name used to filter the list of streams. Returns only the streams of those users that are broadcasting. You may specify a maximum of 100 login names. To specify multiple names, include the user_login parameter for each user. For example, &user_login=foo&user_login=bar.
    #[serde(skip_serializing_if = "Option::is_none")]
    user_login: Option<String>,

    /// A game (category) ID used to filter the list of streams. Returns only the streams that are broadcasting the game (category). You may specify a maximum of 100 IDs. To specify multiple IDs, include the game_id parameter for each game. For example, &game_id=9876&game_id=5432.
    #[serde(skip_serializing_if = "Option::is_none")]
    game_id: Option<String>,

    /// The type of stream to filter the list of streams by. Possible values are:
    ///
    /// - all
    /// - live
    // The default is all.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_: Option<&'static str>,

    /// A language code used to filter the list of streams. Returns only streams that broadcast in the specified language. Specify the language using an ISO 639-1 two-letter language code or other if the broadcast uses a language not in the list of supported stream languages.
    ///
    // You may specify a maximum of 100 language codes. To specify multiple languages, include the language parameter for each language. For example, &language=de&language=fr.
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,

    /// The maximum number of items to return per page in the response. The minimum page size is 1 item per page and the maximum is 100 items per page. The default is 20.
    #[serde(skip_serializing_if = "Option::is_none")]
    first: Option<u32>,

    /// The cursor used to get the previous page of results. The Pagination object in the response contains the cursor’s value. Read More
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<Secret>,

    /// The cursor used to get the next page of results. The Pagination object in the response contains the cursor’s value. Read More
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<Secret>,
}

impl StreamsRequest {
    const EMPTY: Self = Self {
        user_id: None,
        user_login: None,
        game_id: None,
        type_: None,
        language: None,
        first: None,
        before: None,
        after: None,
    };

    pub fn user_id(user_id: String) -> Self {
        Self {
            user_id: Some(user_id),
            ..Self::EMPTY
        }
    }
}

impl Request for StreamsRequest {
    type Encoding = UrlParamEncoding;
    type Response = StreamsResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/streams")
    }
}

#[derive(Debug, Deserialize)]
pub struct StreamsResponse {
    /// The list of streams.
    pub data: Vec<Stream>,

    /// The information used to page through the list of results. The object is empty if there are no more pages left to page through. Read More
    pub pagination: Pagination,
}

impl StreamsResponse {
    pub fn into_stream(mut self) -> Option<Stream> {
        if self.data.len() > 1 {
            unreachable!("mulitple streams returned");
        }
        self.data.pop()
    }
}

#[derive(Debug, Deserialize)]
pub struct Stream {
    /// An ID that identifies the stream. You can use this ID later to look up the video on demand (VOD).
    pub id: String,

    /// The ID of the user that’s broadcasting the stream.
    pub user_id: String,

    /// The user’s login name.
    pub user_login: String,

    /// The user’s display name.
    pub user_name: String,

    /// The ID of the category or game being played.
    pub game_id: String,

    /// The name of the category or game being played.
    pub game_name: String,

    /// The type of stream. Possible values are:
    ///
    /// - live
    ///
    /// If an error occurs, this field is set to an empty string.
    #[serde(rename = "type")]
    pub type_: StreamType,

    /// The stream’s title. Is an empty string if not set.
    pub title: String,

    ///  The tags applied to the stream.
    pub tags: Vec<String>,

    /// The number of users watching the stream.
    pub viewer_count: u32,

    /// The UTC date and time (in RFC3339 format) of when the broadcast began.
    pub started_at: DateTime<Utc>,

    /// The language that the stream uses. This is an ISO 639-1 two-letter language code or other if the stream uses a language not in the list of supported stream languages.
    pub language: String,

    /// A URL to an image of a frame from the last 5 minutes of the stream. Replace the width and height placeholders in the URL ({width}x{height}) with the size of the image you want, in pixels.
    pub thumbnail_url: String,

    /// IMPORTANT As of February 28, 2023, this field is deprecated and returns only an empty array. If you use this field, please update your code to use the tags field.
    ///
    /// The list of tags that apply to the stream. The list contains IDs only when the channel is steaming live. For a list of possible tags, see List of All Tags. The list doesn’t include Category Tags.
    #[expect(dead_code)]
    tag_ids: Vec<String>,

    /// A Boolean value that indicates whether the stream is meant for mature audiences.
    pub is_mature: bool,
}
