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

use crate::dto::recommendations::{
    QueryAnimeRecommendationsCommand, QueryMangaRecommendationsCommand,
};
use crate::endpoint::Endpoint;
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::recommendations as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("recommendations");

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
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(state, uri, move || async move {
            kuukan_mal::api::recommendations::get_recent_recommendations(
                &mal,
                recommendation_type,
                None,
            )
            .await
        })
        .await?;

    let data = resource::recommendations(&cached.payload);
    Ok(cached.render(data))
}
