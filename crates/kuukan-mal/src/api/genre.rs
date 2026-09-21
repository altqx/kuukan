//! `api/genre.rs`: the frozen genre entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::genre::{
    AnimeGenreListParser, AnimeGenreParser, MangaGenreListParser, MangaGenreParser,
};
use crate::request::genre::{
    AnimeGenreRequest, AnimeGenresRequest, MangaGenreRequest, MangaGenresRequest,
};
use crate::source::MalSource;

/// `MalClient::getAnimeGenres(AnimeGenresRequest $request)` (full list).
pub async fn get_anime_genres(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<AnimeGenreListParser>(client, AnimeGenresRequest::new()).await
}

/// `MalClient::getAnimeGenre(AnimeGenreRequest $request)` (genre listing).
pub async fn get_anime_genre(
    client: &dyn MalSource,
    id: i64,
    page: u64,
) -> Result<Value, MalError> {
    super::fetch_and_parse::<AnimeGenreParser>(client, AnimeGenreRequest::new(id, page)).await
}

/// `MalClient::getMangaGenres(MangaGenresRequest $request)` (full list).
pub async fn get_manga_genres(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<MangaGenreListParser>(client, MangaGenresRequest::new()).await
}

/// `MalClient::getMangaGenre(MangaGenreRequest $request)` (genre listing).
pub async fn get_manga_genre(
    client: &dyn MalSource,
    id: i64,
    page: u64,
) -> Result<Value, MalError> {
    super::fetch_and_parse::<MangaGenreParser>(client, MangaGenreRequest::new(id, page)).await
}
