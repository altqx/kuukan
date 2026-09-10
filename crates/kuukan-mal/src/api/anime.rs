//! `api/anime.rs`: the frozen anime entry points (mirrors `Jikan\MyAnimeList\MalClient`).
//!
//! Each function mirrors the matching `Jikan\MyAnimeList\MalClient` method:
//! build the request path, scrape the page, run the parser and return the
//! JMS-shaped `serde_json::Value` (nulls preserved, empty arrays kept).
//!
//! Error mapping matches `MalClient`:
//! - HTTP failures surface as [`MalError::BadResponse`] from the client,
//! - parser failures are wrapped with [`MalError::parse_failed`]
//!   (`ParserException::fromRequest`, message `Failed to parse '<path>'`),
//! - `get_anime_episodes` maps a MAL 404 to an empty `Episodes` model
//!   (`MalClient::getAnimeEpisodes()`),
//! - `get_anime_episode` maps the missing-marker signal of the parser to a
//!   `404 on <path>` [`MalError::BadResponse`] (`MalClient::getAnimeEpisode()`).

use serde_json::{json, Value};

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::anime::{
    AnimeEpisodeParser, AnimeError, AnimeParser, AnimeRecentlyUpdatedByUsersParser,
    AnimeReviewsParser, AnimeStatsParser, CharactersAndStaffParser, EpisodesParser, MoreInfoParser,
    VideosParser,
};
use crate::parser::{common, forum, news};
use crate::request::anime as req;
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getAnime(AnimeRequest $request)`.
pub async fn get_anime(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = req::AnimeRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    AnimeParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeEpisodes(AnimeEpisodesRequest $request)`.
///
/// The episode page returns 404 when there are no results; PHP returns an empty
/// `Episodes` model in that case.
pub async fn get_anime_episodes(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
) -> Result<Value, MalError> {
    let path = req::AnimeEpisodesRequest::new(id, page.unwrap_or(1) as i64).path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(MalError::BadResponse { status: 404, .. }) => {
            return Ok(json!({
                "results": [],
                "has_next_page": false,
                "last_visible_page": 1,
            }));
        }
        Err(error) => return Err(error),
    };
    EpisodesParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeEpisode(AnimeEpisodeRequest $request)`.
pub async fn get_anime_episode(
    client: &MalClient,
    id: i64,
    episode_id: i64,
) -> Result<Value, MalError> {
    let path = req::AnimeEpisodeRequest::new(id, episode_id).path();
    let doc = client.get_html(&path).await?;
    match AnimeEpisodeParser::new(doc).get_model() {
        Ok(model) => Ok(model),
        Err(AnimeError::EpisodeNotFound) => Err(MalError::BadResponse {
            status: 404,
            url: path,
        }),
        Err(AnimeError::Parse(error)) => Err(parse_failed(&path, error)),
    }
}

/// `MalClient::getAnimeVideos(AnimeVideosRequest $request)`.
pub async fn get_anime_videos(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = req::AnimeVideosRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    VideosParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeVideosEpisodes(AnimeVideosEpisodesRequest $request)`.
pub async fn get_anime_videos_episodes(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
) -> Result<Value, MalError> {
    let path = req::AnimeVideosEpisodesRequest::new(id, page.unwrap_or(1) as i64).path();
    let doc = client.get_html(&path).await?;
    VideosParser::new(doc)
        .get_results_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeCharactersAndStaff(AnimeCharactersAndStaffRequest)`.
pub async fn get_anime_characters_and_staff(
    client: &MalClient,
    id: i64,
) -> Result<Value, MalError> {
    let path = req::AnimeCharactersAndStaffRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    CharactersAndStaffParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimePictures(AnimePicturesRequest $request)` (array).
pub async fn get_anime_pictures(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = req::AnimePicturesRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    common::pictures_page(&doc)
        .map(Value::Array)
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeMoreInfo(AnimeMoreInfoRequest $request)`.
///
/// `MalClient` returns the bare string; `AnimeMoreInfoLookupHandler` stores it
/// as `{"moreinfo": string|null}` (the shape `MoreInfoResource` reads), so the
/// wrapper is part of the returned document (same as `api::manga`).
pub async fn get_anime_more_info(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = req::AnimeMoreInfoRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    let more_info = MoreInfoParser::new(doc)
        .get_more_info()
        .map_err(|error| parse_failed(&path, error))?;
    Ok(json!({ "moreinfo": more_info }))
}

/// `MalClient::getAnimeStats(AnimeStatsRequest $request)`.
pub async fn get_anime_stats(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = req::AnimeStatsRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    AnimeStatsParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeForum(AnimeForumRequest $request)` (array of topics).
pub async fn get_anime_forum(
    client: &MalClient,
    id: i64,
    topic: Option<&str>,
) -> Result<Value, MalError> {
    let path = req::AnimeForumRequest::new(id, topic).path();
    let doc = client.get_html(&path).await?;
    forum::parse_forum(&doc)
        .map(Value::Array)
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getNewsList(AnimeNewsRequest $request)`.
///
/// Returns the `NewsListItem[]` payload (`parse_news`), like `api::manga`.
pub async fn get_anime_news(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
) -> Result<Value, MalError> {
    let path = req::AnimeNewsRequest::new(id, page.unwrap_or(1) as i64).path();
    let doc = client.get_html(&path).await?;
    news::parse_news(&doc)
        .map(Value::Array)
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeRecentlyUpdatedByUsers(...)`.
pub async fn get_anime_recently_updated_by_users(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
) -> Result<Value, MalError> {
    let path = req::AnimeRecentlyUpdatedByUsersRequest::new(id, page.unwrap_or(1) as i64).path();
    let doc = client.get_html(&path).await?;
    AnimeRecentlyUpdatedByUsersParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeRecommendations(AnimeRecommendationsRequest)` (array).
pub async fn get_anime_recommendations(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = req::AnimeRecommendationsRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    common::recommendations(&doc)
        .map(Value::Array)
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeReviews(AnimeReviewsRequest $request)`.
pub async fn get_anime_reviews(
    client: &MalClient,
    id: i64,
    page: Option<u64>,
    sort: Option<&str>,
    spoilers: Option<bool>,
    preliminary: Option<bool>,
) -> Result<Value, MalError> {
    let path = req::AnimeReviewsRequest::new(
        id,
        page.unwrap_or(1) as i64,
        sort.unwrap_or(req::REVIEWS_SORT_MOST_VOTED),
        spoilers.unwrap_or(true),
        preliminary.unwrap_or(true),
    )
    .path();
    let doc = client.get_html(&path).await?;
    AnimeReviewsParser::new(doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeGenres(AnimeGenresRequest $request)` (full genre list).
pub async fn get_anime_genres(client: &MalClient) -> Result<Value, MalError> {
    crate::api::genre::get_anime_genres(client).await
}

/// `MalClient::getAnimeGenre(AnimeGenreRequest $request)` (genre listing).
pub async fn get_anime_genre(client: &MalClient, id: i64, page: u64) -> Result<Value, MalError> {
    crate::api::genre::get_anime_genre(client, id, page).await
}
