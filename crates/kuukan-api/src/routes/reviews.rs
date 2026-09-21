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

use crate::dto::base::ReviewRequestParams;
use crate::dto::reviews::{QueryAnimeReviewsCommand, QueryMangaReviewsCommand};
use crate::endpoint::Endpoint;
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::reviews as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("reviews");

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
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(state, uri, move || async move {
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
    Ok(cached.render(data))
}
