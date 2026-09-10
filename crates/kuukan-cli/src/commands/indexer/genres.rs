//! `indexer:genres` — port of `App\Console\Commands\Indexer\GenreIndexer`.
//!
//! Unlike jikan (which shards genres into four Mongo collections per media
//! type), kuukan stores the full genre-list documents in the fingerprint cache
//! with `CACHE_GENRE_EXPIRE`; the `/genres/anime` and `/genres/manga` routes
//! select the requested subset from them.

use anyhow::Result;
use kuukan_api::config::CacheCategory;
use kuukan_api::state::AppState;
use serde_json::Value;

use super::common;
use super::IndexerOptions;

/// URI of the anime genre list (route fingerprint base).
pub const ANIME_GENRES_URI: &str = "/v1/genres/anime";
/// URI of the manga genre list (route fingerprint base).
pub const MANGA_GENRES_URI: &str = "/v1/genres/manga";

pub async fn run(options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, _options: IndexerOptions) -> Result<()> {
    let anime = kuukan_mal::api::genre::get_anime_genres(&state.mal).await?;
    let manga = kuukan_mal::api::genre::get_manga_genres(&state.mal).await?;

    let (anime, manga) = store_genre_documents(state, &anime, &manga).await?;
    tracing::info!(anime = %anime, manga = %manga, "genre lists cached");
    Ok(())
}

/// Cache both genre documents under the fingerprints their routes compute.
///
/// Split out from [`run_with_state`] so tests can exercise the cache writes
/// with constructed payloads.
pub async fn store_genre_documents(
    state: &AppState,
    anime: &Value,
    manga: &Value,
) -> Result<(String, String)> {
    let ttl = state.config.cache_ttl(CacheCategory::Genre) as i64;

    let anime_fingerprint =
        common::put_cached_document(state, "genres", ANIME_GENRES_URI, ttl, anime.clone()).await?;
    let manga_fingerprint =
        common::put_cached_document(state, "genres", MANGA_GENRES_URI, ttl, manga.clone()).await?;

    Ok((anime_fingerprint, manga_fingerprint))
}

#[cfg(test)]
mod tests {
    use super::super::common::test_support;
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn genre_documents_use_route_fingerprints_and_genre_ttl() {
        let (state, _dir) = test_support::state("genres").await;
        let anime = json!({"genres": [{"mal_id": 1}], "explicit_genres": [], "themes": [], "demographics": []});
        let manga = json!({"genres": [{"mal_id": 2}], "explicit_genres": [], "themes": [], "demographics": []});

        let (anime_fingerprint, manga_fingerprint) =
            store_genre_documents(&state, &anime, &manga).await.unwrap();

        // `/v1/genres/anime` and `/v1/genres/manga`, request type `genres`.
        assert_eq!(
            anime_fingerprint,
            "request:genres:221ee7a02c2cb0a335ddebfa11e7d913287f29ea"
        );
        assert_eq!(
            manga_fingerprint,
            "request:genres:41642d4e2f988457e76a5baa1a04c03c67316737"
        );

        let cached = state
            .store
            .get_cache(&anime_fingerprint)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cached.payload["genres"][0]["mal_id"], 1);
        let ttl = state.config.cache_ttl(CacheCategory::Genre) as i64;
        assert_eq!(cached.expires_at, Some(cached.modified_at + ttl));
    }
}
