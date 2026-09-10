//! `/producers` routes — port of `ProducerController` (lookup endpoints).

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::config::CacheCategory;
use kuukan_core::envelope;
use crate::dto::producer::{ProducerExternalLookupCommand, ProducerFullLookupCommand, ProducerLookupCommand};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::producer as resource;
use crate::services::scrape::{entity_or_scrape, fingerprint, request_uri, CachedPayload};
use crate::state::AppState;
use kuukan_store::EntityKind;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/producers/{id}", get(main))
        .route("/producers/{id}/full", get(full))
        .route("/producers/{id}/external", get(external))
}

/// Shared lookup: validate, fetch the producer entity (scrape on miss) and
/// compute the endpoint fingerprint/ttl.
async fn load(
    state: &AppState,
    uri: &axum::http::Uri,
    id: i64,
) -> Result<(CachedPayload, u64, String), ApiErrorResponse> {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);
    let mal = state.mal.clone();
    let cached = entity_or_scrape(state, EntityKind::Producer, id, ttl, move || async move {
        kuukan_mal::api::producer::get_producer(&mal, id, 1).await
    })
    .await?;
    Ok((cached, ttl, uri))
}

async fn main(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducerLookupCommand::parse(id, &query)?;
    let (cached, ttl, uri) = load(&state, &uri, command.id).await?;
    let data = envelope::data(resource::producer(&cached.payload));
    Ok(json_with_cache_flags(
        data,
        &fingerprint("producers", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn full(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducerFullLookupCommand::parse(id, &query)?;
    let (cached, ttl, uri) = load(&state, &uri, command.id).await?;
    let data = envelope::data(resource::producer_full(&cached.payload));
    Ok(json_with_cache_flags(
        data,
        &fingerprint("producers", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn external(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducerExternalLookupCommand::parse(id, &query)?;
    let (cached, ttl, uri) = load(&state, &uri, command.id).await?;
    let data = envelope::data(resource::external_links(&cached.payload));
    Ok(json_with_cache_flags(
        data,
        &fingerprint("producers", &uri),
        cached.modified_at,
        ttl,
    ))
}
