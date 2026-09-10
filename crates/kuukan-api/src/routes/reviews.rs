//! `/reviews` routes — port of `ReviewsController` and
//! `QueryAnimeReviewsHandler`/`QueryMangaReviewsHandler`.
//!
//! Both endpoints are global review lists served from the scraper cache
//! (`QueryReviewsHandler`) and rendered through the default `ResultsResource`.
//! `sort` defaults to `mostVoted`, `spoilers`/`preliminary` to `false`
//! (`ResolvesMediaReviewParams`).

use axum::extract::{OriginalUri, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::config::CacheCategory;
use crate::dto::base::ReviewRequestParams;
use crate::dto::reviews::{QueryAnimeReviewsCommand, QueryMangaReviewsCommand};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::reviews as resource;
use crate::services::scrape::{cache_or_scrape, fingerprint, request_uri};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/reviews/anime", get(anime))
        .route("/reviews/manga", get(manga))
}

async fn anime(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryAnimeReviewsCommand::parse(&query)?;
    load(&state, &uri, "anime", command.review_request_params()).await
}

async fn manga(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryMangaReviewsCommand::parse(&query)?;
    load(&state, &uri, "manga", command.review_request_params()).await
}

async fn load(
    state: &AppState,
    uri: &axum::http::Uri,
    review_type: &'static str,
    params: ReviewRequestParams,
) -> Result<Response, ApiErrorResponse> {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);

    let mal = state.mal.clone();
    let cached = cache_or_scrape(state, "reviews", &uri, ttl, move || async move {
        kuukan_mal::api::reviews::get_reviews(
            &mal,
            review_type,
            Some(params.page),
            params.sort.as_str(),
            params.spoilers,
            params.preliminary,
        )
        .await
    })
    .await?;

    let data = resource::reviews(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("reviews", &uri),
        cached.modified_at,
        ttl,
    ))
}
