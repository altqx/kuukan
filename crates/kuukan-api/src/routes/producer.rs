//! `/producers` routes — port of `ProducerController` (lookup endpoints).

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::dto::producer::{
    ProducerExternalLookupCommand, ProducerFullLookupCommand, ProducerLookupCommand,
};
use crate::endpoint::{Cached, Endpoint};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::producer as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("producers");
use kuukan_core::envelope;
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
) -> Result<Cached, ApiErrorResponse> {
    let mal = state.mal.clone();
    ENDPOINT
        .entity(state, uri, EntityKind::Producer, id, move || async move {
            kuukan_mal::api::producer::get_producer(&mal, id, 1).await
        })
        .await
}

async fn main(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducerLookupCommand::parse(id, &query)?;
    let cached = load(&state, &uri, command.id).await?;
    let data = envelope::data(resource::producer(&cached.payload));
    Ok(cached.render(data))
}

async fn full(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducerFullLookupCommand::parse(id, &query)?;
    let cached = load(&state, &uri, command.id).await?;
    let data = envelope::data(resource::producer_full(&cached.payload));
    Ok(cached.render(data))
}

async fn external(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducerExternalLookupCommand::parse(id, &query)?;
    let cached = load(&state, &uri, command.id).await?;
    let data = envelope::data(resource::external_links(&cached.payload));
    Ok(cached.render(data))
}
