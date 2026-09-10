//! `api/genre.rs`: the frozen genre entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::genre::{
    AnimeGenreListParser, AnimeGenreParser, MangaGenreListParser, MangaGenreParser,
};
use crate::request::genre::{
    AnimeGenreRequest, AnimeGenresRequest, MangaGenreRequest, MangaGenresRequest,
};
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getAnimeGenres(AnimeGenresRequest $request)` (full list).
pub async fn get_anime_genres(client: &MalClient) -> Result<Value, MalError> {
    let path = AnimeGenresRequest::new().path();
    let doc = client.get_html(&path).await?;
    AnimeGenreListParser::new(doc)
        .model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getAnimeGenre(AnimeGenreRequest $request)` (genre listing).
pub async fn get_anime_genre(client: &MalClient, id: i64, page: u64) -> Result<Value, MalError> {
    let path = AnimeGenreRequest::new(id, page).path();
    let doc = client.get_html(&path).await?;
    AnimeGenreParser::new(doc)
        .model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getMangaGenres(MangaGenresRequest $request)` (full list).
pub async fn get_manga_genres(client: &MalClient) -> Result<Value, MalError> {
    let path = MangaGenresRequest::new().path();
    let doc = client.get_html(&path).await?;
    MangaGenreListParser::new(doc)
        .model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getMangaGenre(MangaGenreRequest $request)` (genre listing).
pub async fn get_manga_genre(client: &MalClient, id: i64, page: u64) -> Result<Value, MalError> {
    let path = MangaGenreRequest::new(id, page).path();
    let doc = client.get_html(&path).await?;
    MangaGenreParser::new(doc)
        .model()
        .map_err(|error| parse_failed(&path, error))
}
