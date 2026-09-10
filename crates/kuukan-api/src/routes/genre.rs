//! `/genres` routes — port of `GenreController`.
//!
//! Jikan's genre list endpoints are served from the genre collections without
//! cache flags (the response has Symfony's default `Cache-Control: no-cache,
//! private`). Kuukan caches the whole list document under the request
//! fingerprint (populated on demand or by `kuukan indexer genres`) and selects
//! the requested subset, mirroring `GenreListHandler`.

use axum::extract::{OriginalUri, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use kuukan_core::enums::GenreFilter;
use serde_json::Value;

use crate::config::CacheCategory;
use crate::dto::anime::AnimeGenreListCommand;
use crate::dto::manga::MangaGenreListCommand;
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::genre as resource;
use crate::services::scrape::cache_or_scrape;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/genres/anime", get(anime))
        .route("/genres/manga", get(manga))
}

async fn anime(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeGenreListCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Genre);
    // Genre lists are media-wide documents: pagination query parameters do not
    // change the cache key (PHP serves them from unkeyed collections).
    let canonical = uri.path().to_string();

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "genres", &canonical, ttl, move || async move {
        kuukan_mal::api::genre::get_anime_genres(&mal).await
    })
    .await?;

    let items = select_genres(&cached.payload, command.filter);
    Ok(Json(resource::genre_list_response(&items)).into_response())
}

async fn manga(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaGenreListCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Genre);
    // Genre lists are media-wide documents: pagination query parameters do not
    // change the cache key (PHP serves them from unkeyed collections).
    let canonical = uri.path().to_string();

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "genres", &canonical, ttl, move || async move {
        kuukan_mal::api::genre::get_manga_genres(&mal).await
    })
    .await?;

    let items = select_genres(&cached.payload, command.filter);
    Ok(Json(resource::genre_list_response(&items)).into_response())
}

/// Mirrors `GenreListHandler::handle`: `genres`, `explicit_genres`, `themes`,
/// `demographics`, or all four concatenated (in that order).
pub(crate) fn select_genres(payload: &Value, filter: Option<GenreFilter>) -> Vec<Value> {
    let arr = |key: &str| -> Vec<Value> {
        payload
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    match filter {
        Some(GenreFilter::Genres) => arr("genres"),
        Some(GenreFilter::ExplicitGenres) => arr("explicit_genres"),
        Some(GenreFilter::Themes) => arr("themes"),
        Some(GenreFilter::Demographics) => arr("demographics"),
        None => {
            let mut all = arr("genres");
            all.extend(arr("explicit_genres"));
            all.extend(arr("themes"));
            all.extend(arr("demographics"));
            all
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn select_genres_matches_php_concat_order() {
        let payload = json!({
            "genres": [{"mal_id": 1}],
            "explicit_genres": [{"mal_id": 12}],
            "themes": [{"mal_id": 50}],
            "demographics": [{"mal_id": 27}],
        });
        let all = select_genres(&payload, None);
        assert_eq!(all.len(), 4);
        assert_eq!(all[0]["mal_id"], 1);
        assert_eq!(all[1]["mal_id"], 12);
        assert_eq!(all[2]["mal_id"], 50);
        assert_eq!(all[3]["mal_id"], 27);

        let themes = select_genres(&payload, Some(GenreFilter::Themes));
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0]["mal_id"], 50);
    }
}
