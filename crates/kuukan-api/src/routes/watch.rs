//! `/watch` routes — port of `WatchController` and the
//! `QueryRecentlyAdded*`/`QueryPopular*` handlers.
//!
//! All four endpoints are scraper caches (`RequestHandlerWithScraperCache`):
//! the MAL result is stored under the request fingerprint and rendered through
//! the default `ResultsResource`.

use axum::extract::{OriginalUri, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::config::CacheCategory;
use crate::dto::watch::{
    QueryPopularEpisodesCommand, QueryPopularPromoVideosCommand, QueryRecentlyAddedEpisodesCommand,
    QueryRecentlyAddedPromoVideosCommand,
};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::watch as resource;
use crate::services::scrape::{cache_or_scrape, fingerprint, request_uri};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/watch/episodes", get(recent_episodes))
        .route("/watch/episodes/popular", get(popular_episodes))
        .route("/watch/promos", get(recent_promos))
        .route("/watch/promos/popular", get(popular_promos))
}

async fn recent_episodes(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryRecentlyAddedEpisodesCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "watch", &uri, ttl, move || async move {
        kuukan_mal::api::watch::get_recent_episodes(&mal).await
    })
    .await?;

    let data = resource::watch_episodes(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("watch", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn popular_episodes(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryPopularEpisodesCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "watch", &uri, ttl, move || async move {
        kuukan_mal::api::watch::get_popular_episodes(&mal).await
    })
    .await?;

    let data = resource::watch_episodes(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("watch", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn recent_promos(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryRecentlyAddedPromoVideosCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);

    let mal = state.mal.clone();
    let page = command.page;
    let cached = cache_or_scrape(&state, "watch", &uri, ttl, move || async move {
        kuukan_mal::api::watch::get_recent_promotional_videos(&mal, page).await
    })
    .await?;

    let data = resource::watch_promos(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("watch", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn popular_promos(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryPopularPromoVideosCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "watch", &uri, ttl, move || async move {
        kuukan_mal::api::watch::get_popular_promotional_videos(&mal).await
    })
    .await?;

    let data = resource::watch_promos(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("watch", &uri),
        cached.modified_at,
        ttl,
    ))
}
