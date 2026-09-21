//! `/people` routes — port of `PersonController`.
//!
//! Wiring mirrors the PHP handler/repository bindings
//! (`AppServiceProvider::requestHandlersWithScraperService`):
//!
//! - `main`, `full`, `anime`, `manga` and `voices` extend `ItemLookupHandler`
//!   bound to the **people** repository, so they serve the stored person
//!   entity (scraping the full `PersonRequest` document on miss/expiry);
//! - `pictures` extends `RequestHandlerWithScraperCache` bound to the
//!   `people_pictures` documents, i.e. a fingerprint-keyed cache of
//!   `getPersonPictures`.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::Value;

use crate::dto::person::{
    PersonAnimeLookupCommand, PersonFullLookupCommand, PersonLookupCommand,
    PersonMangaLookupCommand, PersonPicturesLookupCommand, PersonVoicesLookupCommand,
};
use crate::endpoint::{Cached, Endpoint};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::person as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("people");
use kuukan_core::envelope;
use kuukan_store::EntityKind;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/people/{id}", get(main))
        .route("/people/{id}/full", get(full))
        .route("/people/{id}/anime", get(anime))
        .route("/people/{id}/manga", get(manga))
        .route("/people/{id}/voices", get(voices))
        .route("/people/{id}/pictures", get(pictures))
}

/// Shared entity lookup: validate, fetch the person entity (scrape on miss)
/// and compute the endpoint fingerprint/ttl.
async fn load_person(
    state: &AppState,
    uri: &axum::http::Uri,
    id: i64,
) -> Result<Cached, ApiErrorResponse> {
    let mal = state.mal.clone();
    ENDPOINT
        .entity(state, uri, EntityKind::Person, id, move || async move {
            kuukan_mal::api::person::get_person(&mal, id).await
        })
        .await
}

/// `$results->get(<key>)` as a JSON array (`[]` when the key is absent).
fn items(payload: &Value, key: &str) -> Vec<Value> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

async fn main(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PersonLookupCommand::parse(id, &query)?;
    let cached = load_person(&state, &uri, command.id).await?;
    let data = envelope::data(resource::person(&cached.payload));
    Ok(cached.render(data))
}

async fn full(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PersonFullLookupCommand::parse(id, &query)?;
    let cached = load_person(&state, &uri, command.id).await?;
    let data = envelope::data(resource::person_full(&cached.payload));
    Ok(cached.render(data))
}

async fn anime(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PersonAnimeLookupCommand::parse(id, &query)?;
    let cached = load_person(&state, &uri, command.id).await?;
    let data = envelope::data(resource::person_anime_collection(&items(
        &cached.payload,
        "anime_staff_positions",
    )));
    Ok(cached.render(data))
}

async fn manga(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PersonMangaLookupCommand::parse(id, &query)?;
    let cached = load_person(&state, &uri, command.id).await?;
    let data = envelope::data(resource::person_manga_collection(&items(
        &cached.payload,
        "published_manga",
    )));
    Ok(cached.render(data))
}

async fn voices(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PersonVoicesLookupCommand::parse(id, &query)?;
    let cached = load_person(&state, &uri, command.id).await?;
    let data = envelope::data(resource::person_voices_collection(&items(
        &cached.payload,
        "voice_acting_roles",
    )));
    Ok(cached.render(data))
}

async fn pictures(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PersonPicturesLookupCommand::parse(id, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            let pictures = kuukan_mal::api::person::get_person_pictures(&mal, command.id).await?;
            Ok(serde_json::json!({ "pictures": pictures }))
        })
        .await?;
    let data = envelope::data(resource::person_pictures(&cached.payload));
    Ok(cached.render(data))
}
