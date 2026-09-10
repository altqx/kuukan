//! `api/search.rs`: the frozen search entry points.
//!
//! Each function mirrors the matching `Jikan\MyAnimeList\MalClient` method:
//! build the request path, scrape the page, run the parser and return the
//! JMS-shaped `serde_json::Value`.
//!
//! `MalClient::getCharacterSearch()` / `getPersonSearch()` return an empty
//! model on a MAL 404 and `getUserSearch()` returns an empty `UserSearch`;
//! that behavior is kept here.

use serde_json::{json, Value};

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::search::{
    AnimeSearchParser, CharacterSearchParser, MangaSearchParser, PersonSearchParser,
    UserSearchParser,
};
use crate::request::search::{
    AnimeSearchRequest, CharacterSearchRequest, MangaSearchRequest, PersonSearchRequest,
    UserSearchRequest,
};
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// Empty `{results, has_next_page, last_visible_page}` model (`::mock()`).
fn empty_search_model() -> Value {
    json!({
        "results": [],
        "has_next_page": false,
        "last_visible_page": 1,
    })
}

/// `MalClient::getAnimeSearch(AnimeSearchRequest $request)`.
pub async fn get_anime_search(
    client: &MalClient,
    request: &AnimeSearchRequest,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = client.get_html(&path).await?;
    AnimeSearchParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeSearchAlt(AnimeSearchRequest $request)`.
///
/// PHP reuses `AnimeSearchParser::getModel()` for this method (there is no
/// separate `AnimeSearchAlt` model), so the payload is identical to
/// [`get_anime_search`].
pub async fn get_anime_search_alt(
    client: &MalClient,
    request: &AnimeSearchRequest,
) -> Result<Value, MalError> {
    get_anime_search(client, request).await
}

/// `MalClient::getMangaSearch(MangaSearchRequest $request)`.
pub async fn get_manga_search(
    client: &MalClient,
    request: &MangaSearchRequest,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = client.get_html(&path).await?;
    MangaSearchParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getCharacterSearch(CharacterSearchRequest $request)`.
///
/// A MAL 404 becomes `CharacterSearch::mock()` (empty results).
pub async fn get_character_search(
    client: &MalClient,
    request: &CharacterSearchRequest,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(error) if error.is_not_found() => return Ok(empty_search_model()),
        Err(error) => return Err(error),
    };
    CharacterSearchParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getPersonSearch(PersonSearchRequest $request)`.
///
/// A MAL 404 becomes `PersonSearch::mock()` (empty results).
pub async fn get_person_search(
    client: &MalClient,
    request: &PersonSearchRequest,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(error) if error.is_not_found() => return Ok(empty_search_model()),
        Err(error) => return Err(error),
    };
    PersonSearchParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getUserSearch(UserSearchRequest $request)`.
///
/// A MAL 404 becomes an empty `UserSearch` (the "no results" case).
pub async fn get_user_search(
    client: &MalClient,
    request: &UserSearchRequest,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(error) if error.is_not_found() => return Ok(empty_search_model()),
        Err(error) => return Err(error),
    };
    UserSearchParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
