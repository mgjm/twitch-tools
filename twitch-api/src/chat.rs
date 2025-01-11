use serde::{Deserialize, Serialize};

use crate::client::{Request, UrlParamEncoding};

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
