//! `/characters` routes — port of `CharacterController`.
//!
//! Wiring mirrors the PHP handler/repository bindings
//! (`AppServiceProvider::requestHandlersWithScraperService`):
//!
//! - `main`, `full`, `anime`, `manga` and `voices` extend `ItemLookupHandler`
//!   bound to the **characters** repository, so they serve the stored
//!   character entity (scraping the full `CharacterRequest` document on
//!   miss/expiry). `anime`/`manga`/`voices` only read a slice of that same
//!   document — they are *not* fingerprint caches;
//! - `pictures` extends `RequestHandlerWithScraperCache` bound to the
//!   `characters_pictures` documents, i.e. a fingerprint-keyed cache of
//!   `getCharacterPictures`.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::Value;

use crate::dto::character::{
    CharacterAnimeLookupCommand, CharacterFullLookupCommand, CharacterLookupCommand,
    CharacterMangaLookupCommand, CharacterPicturesLookupCommand, CharacterVoicesLookupCommand,
};
use crate::endpoint::{Cached, Endpoint};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::character as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("characters");
use kuukan_core::envelope;
use kuukan_store::EntityKind;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/characters/{id}", get(main))
        .route("/characters/{id}/full", get(full))
        .route("/characters/{id}/anime", get(anime))
        .route("/characters/{id}/manga", get(manga))
        .route("/characters/{id}/voices", get(voices))
        .route("/characters/{id}/pictures", get(pictures))
}

/// Shared entity lookup: validate, fetch the character entity (scrape on
/// miss) and compute the endpoint fingerprint/ttl.
async fn load_character(
    state: &AppState,
    uri: &axum::http::Uri,
    id: i64,
) -> Result<Cached, ApiErrorResponse> {
    let mal = state.mal.clone();
    ENDPOINT
        .entity(state, uri, EntityKind::Character, id, move || async move {
            kuukan_mal::api::character::get_character(&mal, id).await
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
    let command = CharacterLookupCommand::parse(id, &query)?;
    let cached = load_character(&state, &uri, command.id).await?;
    let data = envelope::data(resource::character(&cached.payload));
    Ok(cached.render(data))
}

async fn full(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = CharacterFullLookupCommand::parse(id, &query)?;
    let cached = load_character(&state, &uri, command.id).await?;
    let data = envelope::data(resource::character_full(&cached.payload));
    Ok(cached.render(data))
}

async fn anime(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = CharacterAnimeLookupCommand::parse(id, &query)?;
    let cached = load_character(&state, &uri, command.id).await?;
    let data = envelope::data(resource::character_anime_collection(&items(
        &cached.payload,
        "animeography",
    )));
    Ok(cached.render(data))
}

async fn manga(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = CharacterMangaLookupCommand::parse(id, &query)?;
    let cached = load_character(&state, &uri, command.id).await?;
    let data = envelope::data(resource::character_manga_collection(&items(
        &cached.payload,
        "mangaography",
    )));
    Ok(cached.render(data))
}

async fn voices(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = CharacterVoicesLookupCommand::parse(id, &query)?;
    let cached = load_character(&state, &uri, command.id).await?;
    let data = envelope::data(resource::character_seiyuu_collection(&items(
        &cached.payload,
        "voice_actors",
    )));
    Ok(cached.render(data))
}

async fn pictures(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = CharacterPicturesLookupCommand::parse(id, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            let pictures =
                kuukan_mal::api::character::get_character_pictures(&mal, command.id).await?;
            Ok(serde_json::json!({ "pictures": pictures }))
        })
        .await?;
    let data = envelope::data(resource::character_pictures(&cached.payload));
    Ok(cached.render(data))
}
