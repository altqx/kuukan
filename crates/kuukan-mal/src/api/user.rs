//! Frozen user entry points.
//!
//! Every function mirrors the matching `Jikan\MyAnimeList\MalClient` method:
//! it builds the request path, fetches through [`MalClient`] and returns the
//! JMS-shaped payload (`serde_json::Value`) of the PHP model. Parser failures
//! surface as [`MalError::Parse`] with the `ParserException` message
//! (`Failed to parse '<path>'`).

use serde_json::{json, Value};

use crate::client::MalClient;
use crate::error::{MalError, ParseError};
use crate::parser::search::UserSearchParser;
use crate::parser::user::{
    anime_list_item, manga_list_item, ClubParser, FriendsParser, HistoryParser, UserProfileParser,
    UserRecommendationsParser, UserReviewsParser, UsernameByIdParser,
};
use crate::request::user::{
    RecentlyOnlineUsersRequest, UserAnimeListRequest, UserClubsRequest, UserFriendsRequest,
    UserHistoryRequest, UserMangaListRequest, UserProfileRequest, UserRecommendationsRequest,
    UserReviewsRequest, UsernameByIdRequest, USER_LIST_ALL,
};
use crate::request::MalRequest;

/// Run a parser and turn its [`ParseError`] into the PHP `ParserException`
/// message for `path`.
fn parsed<F, T>(path: &str, parse: F) -> Result<Value, MalError>
where
    F: FnOnce() -> Result<T, ParseError>,
    T: Into<Value>,
{
    match parse() {
        Ok(value) => Ok(value.into()),
        Err(error) => Err(MalError::parse_failed(path, error.to_string())),
    }
}

/// `MalClient::getUserProfile()`.
pub async fn get_user_profile(client: &MalClient, username: &str) -> Result<Value, MalError> {
    let request = UserProfileRequest::new(username);
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parsed(&path, || UserProfileParser::new(&doc).get_model())
}

/// `MalClient::getUserFriends()`.
pub async fn get_user_friends(
    client: &MalClient,
    username: &str,
    page: Option<u32>,
) -> Result<Value, MalError> {
    let request = UserFriendsRequest::new(username, page.unwrap_or(1) as i64);
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parsed(&path, || FriendsParser::new(&doc).get_model())
}

/// `MalClient::getUserHistory()`.
///
/// `type_` is `anime`, `manga` or `None` (the PHP request rejects any other
/// value with `InvalidArgumentException`).
pub async fn get_user_history(
    client: &MalClient,
    username: &str,
    type_: Option<&str>,
) -> Result<Value, MalError> {
    let request = UserHistoryRequest::try_new(username, type_).map_err(MalError::Parse)?;
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parsed(&path, || HistoryParser::new(&doc).get_model())
}

/// `MalClient::getUserAnimeList()`.
///
/// `status` defaults to `Constants::USER_ANIME_LIST_ALL` (7), `page` to 1;
/// MAL receives `offset = (page - 1) * 300`.
pub async fn get_user_anime_list(
    client: &MalClient,
    username: &str,
    page: Option<u32>,
    status: Option<i64>,
) -> Result<Value, MalError> {
    let request = UserAnimeListRequest::new(
        username,
        page.unwrap_or(1) as i64,
        status.unwrap_or(USER_LIST_ALL),
    );
    let path = request.path();
    let body = match client.get_json(&path).await {
        Ok(body) => body,
        // PHP `json_decode()` returns null on malformed JSON and the foreach
        // over null yields an empty list (no exception).
        Err(MalError::Json(_)) => return Ok(json!([])),
        Err(error) => return Err(error),
    };
    let items: Vec<Value> = match &body {
        Value::Array(items) => items.iter().map(anime_list_item).collect(),
        Value::Object(map) => map.values().map(anime_list_item).collect(),
        _ => Vec::new(),
    };
    Ok(Value::Array(items))
}

/// `MalClient::getUserMangaList()`.
///
/// `status` defaults to `Constants::USER_MANGA_LIST_ALL` (7), `page` to 1;
/// MAL receives `offset = (page - 1) * 300`.
pub async fn get_user_manga_list(
    client: &MalClient,
    username: &str,
    page: Option<u32>,
    status: Option<i64>,
) -> Result<Value, MalError> {
    let request = UserMangaListRequest::new(
        username,
        page.unwrap_or(1) as i64,
        status.unwrap_or(USER_LIST_ALL),
    );
    let path = request.path();
    let body = match client.get_json(&path).await {
        Ok(body) => body,
        Err(MalError::Json(_)) => return Ok(json!([])),
        Err(error) => return Err(error),
    };
    let items: Vec<Value> = match &body {
        Value::Array(items) => items.iter().map(manga_list_item).collect(),
        Value::Object(map) => map.values().map(manga_list_item).collect(),
        _ => Vec::new(),
    };
    Ok(Value::Array(items))
}

/// `MalClient::getUserClubs()`: a 404 (no clubs) yields an empty array.
pub async fn get_user_clubs(client: &MalClient, username: &str) -> Result<Value, MalError> {
    let request = UserClubsRequest::new(username);
    let path = request.path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(MalError::BadResponse { status: 404, .. }) => return Ok(json!([])),
        Err(error) => return Err(error),
    };
    parsed(&path, || ClubParser::new(&doc).get_clubs())
}

/// `MalClient::getUserRecommendations()`.
pub async fn get_user_recommendations(
    client: &MalClient,
    username: &str,
    page: Option<u32>,
) -> Result<Value, MalError> {
    let request = UserRecommendationsRequest::new(username, Some(page.unwrap_or(1) as i64));
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parsed(&path, || UserRecommendationsParser::new(&doc).get_model())
}

/// `MalClient::getUserReviews()`: a 404 yields `UserReviews::mock()`.
pub async fn get_user_reviews(
    client: &MalClient,
    username: &str,
    page: Option<u32>,
) -> Result<Value, MalError> {
    let request = UserReviewsRequest::new(username, Some(page.unwrap_or(1) as i64));
    let path = request.path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(MalError::BadResponse { status: 404, .. }) => {
            return Ok(json!({
                "results": [],
                "has_next_page": false,
                "last_visible_page": 1,
            }))
        }
        Err(error) => return Err(error),
    };
    parsed(&path, || UserReviewsParser::new(&doc).get_model())
}

/// `MalClient::getUsernameById()`.
pub async fn get_username_by_id(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let request = UsernameByIdRequest::new(id);
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parsed(&path, || UsernameByIdParser::new(&doc).get_user())
}

/// `MalClient::getRecentOnlineUsers()`.
pub async fn get_recent_online_users(client: &MalClient) -> Result<Value, MalError> {
    let request = RecentlyOnlineUsersRequest::new();
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parsed(&path, || UserSearchParser::new(&doc).get_results())
}
