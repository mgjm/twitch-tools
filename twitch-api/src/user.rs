use serde::{Deserialize, Serialize};

use crate::{
    client::{Request, UrlParamEncoding},
    secret::Secret,
};

#[derive(Debug, Serialize)]
pub struct UsersRequest {
    /// The ID of the user to get. To specify more than one user, include the id parameter for each user to get. For example, id=1234&id=5678. The maximum number of IDs you may specify is 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,

    /// The login name of the user to get. To specify more than one user, include the login parameter for each user to get. For example, login=foo&login=bar. The maximum number of login names you may specify is 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    login: Option<String>,
}

impl UsersRequest {
    pub fn me() -> Self {
        Self {
            id: None,
            login: None,
        }
    }

    pub fn id(id: String) -> Self {
        Self {
            id: Some(id),
            login: None,
        }
    }

    pub fn login(login: String) -> Self {
        Self {
            id: None,
            login: Some(login),
        }
    }
}

impl Request for UsersRequest {
    type Encoding = UrlParamEncoding;
    type Response = UsersResponse;

    fn url(&self) -> impl reqwest::IntoUrl {
        twitch_helix!("/users")
    }
}

#[derive(Debug, Deserialize)]
pub struct UsersResponse {
    /// The list of users.
    data: Vec<User>,
}

impl UsersResponse {
    pub fn into_user(mut self) -> Option<User> {
        if self.data.len() > 1 {
            unreachable!("mulitple users returned");
        }
        self.data.pop()
    }
}

#[derive(Debug, Deserialize)]
pub struct User {
    /// An ID that identifies the user.
    pub id: String,

    /// The user’s login name.
    pub login: String,

    /// The user’s display name.
    pub display_name: String,

    /// The type of user. Possible values are:
    ///
    /// admin — Twitch administrator
    /// global_mod
    /// staff — Twitch staff
    /// "" — Normal user
    #[serde(rename = "type")]
    pub type_: UserType,

    /// The type of broadcaster. Possible values are:
    ///
    /// affiliate — An affiliate broadcaster affiliate broadcaster
    /// partner — A partner broadcaster partner broadcaster
    /// "" — A normal broadcaster
    pub broadcaster_type: BroadcasterType,

    /// The user’s description of their channel.
    pub description: String,

    /// A URL to the user’s profile image.
    pub profile_image_url: String,

    /// A URL to the user’s offline image.
    pub offline_image_url: String,

    /// The number of times the user’s channel has been viewed.
    ///
    /// NOTE: This field has been deprecated (see Get Users API endpoint – “view_count” deprecation). Any data in this field is not valid and should not be used.
    view_count: u64,

    /// The user’s verified email address. The object includes this field only if the user access token includes the user:read:email scope.
    ///
    /// If the request contains more than one user, only the user associated with the access token that provided consent will include an email address — the email address for all other users will be empty.
    #[serde(default)]
    pub email: Option<Secret>,

    /// The UTC date and time that the user’s account was created. The timestamp is in RFC3339 format.
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub enum UserType {
    #[serde(rename = "")]
    Normal,

    #[serde(rename = "staff")]
    Staff,

    #[serde(rename = "global_mod")]
    GlobalMod,

    #[serde(rename = "admin")]
    Admin,
}

#[derive(Debug, Deserialize)]
pub enum BroadcasterType {
    #[serde(rename = "")]
    Normal,

    #[serde(rename = "affiliate ")]
    Affiliate,

    #[serde(rename = "partner ")]
    Partner,
}
