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

use crate::dto::watch::{
    QueryPopularEpisodesCommand, QueryPopularPromoVideosCommand, QueryRecentlyAddedEpisodesCommand,
    QueryRecentlyAddedPromoVideosCommand,
};
use crate::endpoint::Endpoint;
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::watch as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("watch");

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
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::watch::get_recent_episodes(&mal).await
        })
        .await?;

    let data = resource::watch_episodes(&cached.payload);
    Ok(cached.render(data))
}

async fn popular_episodes(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryPopularEpisodesCommand::parse(&query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::watch::get_popular_episodes(&mal).await
        })
        .await?;

    let data = resource::watch_episodes(&cached.payload);
    Ok(cached.render(data))
}

async fn recent_promos(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryRecentlyAddedPromoVideosCommand::parse(&query)?;

    let mal = state.mal.clone();
    let page = command.page;
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::watch::get_recent_promotional_videos(&mal, page).await
        })
        .await?;

    let data = resource::watch_promos(&cached.payload);
    Ok(cached.render(data))
}

async fn popular_promos(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryPopularPromoVideosCommand::parse(&query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::watch::get_popular_promotional_videos(&mal).await
        })
        .await?;

    let data = resource::watch_promos(&cached.payload);
    Ok(cached.render(data))
}
