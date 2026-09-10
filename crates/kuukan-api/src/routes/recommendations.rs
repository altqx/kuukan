//! `/recommendations` routes — port of `RecommendationsController` and
//! `QueryAnimeRecommendationsHandler`/`QueryMangaRecommendationsHandler`.
//!
//! Both endpoints are global recommendation lists served from the scraper
//! cache (`QueryRecommendationsHandler`) and rendered through the default
//! `ResultsResource`. The DTOs expose no parameters, so PHP always requests
//! page 1 regardless of the `page` query string (it only changes the cache
//! fingerprint).

use axum::extract::{OriginalUri, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::config::CacheCategory;
use crate::dto::recommendations::{
    QueryAnimeRecommendationsCommand, QueryMangaRecommendationsCommand,
};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::recommendations as resource;
use crate::services::scrape::{cache_or_scrape, fingerprint, request_uri};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/recommendations/anime", get(anime))
        .route("/recommendations/manga", get(manga))
}

async fn anime(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryAnimeRecommendationsCommand::parse(&query)?;
    load(&state, &uri, "anime").await
}

async fn manga(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryMangaRecommendationsCommand::parse(&query)?;
    load(&state, &uri, "manga").await
}

async fn load(
    state: &AppState,
    uri: &axum::http::Uri,
    recommendation_type: &'static str,
) -> Result<Response, ApiErrorResponse> {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);

    let mal = state.mal.clone();
    let cached = cache_or_scrape(state, "recommendations", &uri, ttl, move || async move {
        kuukan_mal::api::recommendations::get_recent_recommendations(
            &mal,
            recommendation_type,
            None,
        )
        .await
    })
    .await?;

    let data = resource::recommendations(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("recommendations", &uri),
        cached.modified_at,
        ttl,
    ))
}
