//! Frozen `get_manga*` entry points.
//!
//! Each function mirrors the matching `Jikan\MyAnimeList\MalClient` method:
//! it builds the request path, fetches the page, runs the parser and returns
//! the JMS-shaped payload. Parser failures become
//! [`MalError::parse_failed`] with the PHP `ParserException` message.

use serde_json::{json, Value};

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::manga::{
    CharactersParser, MangaParser, MangaRecentlyUpdatedByUsersParser, MangaReviewsParser,
    MangaStatsParser, MoreInfoParser,
};
use crate::request::manga::{
    MangaCharactersRequest, MangaForumRequest, MangaMoreInfoRequest, MangaNewsRequest,
    MangaPicturesRequest, MangaRecommendationsRequest, MangaRecentlyUpdatedByUsersRequest,
    MangaRequest, MangaReviewsRequest, MangaStatsRequest,
};
use crate::request::MalRequest;

fn parse_error(path: &str) -> impl FnOnce(crate::error::ParseError) -> MalError + '_ {
    move |err| MalError::parse_failed(path, err.to_string())
}

/// `MalClient::getManga()` — `/manga/{id}`.
pub async fn get_manga(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = MangaRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    MangaParser::new(doc).model().map_err(parse_error(&path))
}

/// `MalClient::getMangaCharacters()` — array of `{character, role}`.
pub async fn get_manga_characters(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = MangaCharactersRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    CharactersParser::new(doc)
        .characters()
        .map(Value::Array)
        .map_err(parse_error(&path))
}

/// `MalClient::getMangaPictures()` — array of image resources.
pub async fn get_manga_pictures(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = MangaPicturesRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    crate::parser::common::pictures_page(&doc)
        .map(Value::Array)
        .map_err(parse_error(&path))
}

/// `MalClient::getMangaMoreInfo()`.
///
/// `MalClient` returns the raw string; `MangaMoreInfoLookupHandler` stores it
/// wrapped as `{"moreinfo": string|null}` (the shape `MoreInfoResource` reads),
/// so the wrapper is part of the returned document.
pub async fn get_manga_more_info(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = MangaMoreInfoRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    let more_info = MoreInfoParser::new(doc)
        .more_info()
        .map_err(parse_error(&path))?;
    Ok(json!({ "moreinfo": more_info }))
}

/// `MalClient::getMangaStats()` — `/manga/{id}/stats`.
pub async fn get_manga_stats(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = MangaStatsRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    MangaStatsParser::new(doc).model().map_err(parse_error(&path))
}

/// `MalClient::getMangaForum()` — array of forum topics.
pub async fn get_manga_forum(
    client: &MalClient,
    id: i64,
    topic: Option<&str>,
) -> Result<Value, MalError> {
    let path = MangaForumRequest::with_topic(id, topic.map(str::to_string)).path();
    let doc = client.get_html(&path).await?;
    crate::parser::forum::parse_forum(&doc)
        .map(Value::Array)
        .map_err(parse_error(&path))
}

/// `MalClient::getNewsList(new MangaNewsRequest(...))`.
pub async fn get_manga_news(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
) -> Result<Value, MalError> {
    // The PHP handler resolves the page with a `1` default before building the
    // request (`$requestParams->get("page", 1)`).
    let path = MangaNewsRequest::with_page(id, Some(page.unwrap_or(1))).path();
    let doc = client.get_html(&path).await?;
    crate::parser::news::parse_news(&doc)
        .map(Value::Array)
        .map_err(parse_error(&path))
}

/// `MalClient::getMangaRecentlyUpdatedByUsers()` — page starts at 1.
pub async fn get_manga_recently_updated_by_users(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
) -> Result<Value, MalError> {
    let path = MangaRecentlyUpdatedByUsersRequest::new(id, page.unwrap_or(1)).path();
    let doc = client.get_html(&path).await?;
    MangaRecentlyUpdatedByUsersParser::new(doc)
        .model()
        .map_err(parse_error(&path))
}

/// `MalClient::getMangaRecommendations()` — array of recommendations.
pub async fn get_manga_recommendations(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = MangaRecommendationsRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    crate::parser::common::recommendations(&doc)
        .map(Value::Array)
        .map_err(parse_error(&path))
}

/// `MalClient::getMangaReviews()`.
pub async fn get_manga_reviews(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
    sort: Option<&str>,
    spoilers: Option<bool>,
    preliminary: Option<bool>,
) -> Result<Value, MalError> {
    let path = MangaReviewsRequest::with_params(id, page, sort, spoilers, preliminary).path();
    let doc = client.get_html(&path).await?;
    MangaReviewsParser::new(doc).model().map_err(parse_error(&path))
}

/// `MalClient::getMangaGenres()` (full genre list).
pub async fn get_manga_genres(client: &MalClient) -> Result<Value, MalError> {
    crate::api::genre::get_manga_genres(client).await
}

/// `MalClient::getMangaGenre()` (genre listing).
pub async fn get_manga_genre(client: &MalClient, id: i64, page: u64) -> Result<Value, MalError> {
    crate::api::genre::get_manga_genre(client, id, page).await
}
