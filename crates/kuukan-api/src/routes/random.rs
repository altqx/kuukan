//! Random routes — `/random/anime`, `/random/manga`, `/random/characters`,
//! `/random/people`, `/random/users`.
//!
//! Port of `RandomController` + `app/Features/QueryRandom*Handler.php`.
//! `Anime`/`Manga` call `random(1, $sfw, $unapproved)`; characters, people and
//! users go through the repository default (`sfw = unapproved = false`). The
//! handlers are plain `JsonResource` responses, so the reference recordings
//! carry Symfony's default cache header only (no Jikan cache flags).
//!
//! An empty result set is a 404: there is no entity to render.

use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use kuukan_core::error::ApiError;
use kuukan_store::EntityKind;

use crate::dto::misc::{
    QueryRandomAnimeCommand, QueryRandomCharacterCommand, QueryRandomMangaCommand,
    QueryRandomPersonCommand, QueryRandomUserCommand,
};
use crate::error::{json_ok, ApiErrorResponse};
use crate::extract::RawQuery;
use crate::resources::{anime, character, manga, person, search as resource};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/random/anime", get(anime))
        .route("/random/manga", get(manga))
        .route("/random/characters", get(characters))
        .route("/random/people", get(people))
        .route("/random/users", get(users))
        .layer(axum::middleware::from_fn(super::search::no_cache_default))
}

async fn anime(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryRandomAnimeCommand::parse(&query)?;
    let payload = random(&state, EntityKind::Anime, command.sfw, command.unapproved).await?;
    // `AnimeResource` reads the Eloquent accessors (`season`, `year`,
    // `broadcast`), which the store keeps in their raw JMS shape.
    let payload = crate::routes::season::materialize_accessors(&payload);
    Ok(json_ok(kuukan_core::envelope::data(anime::anime(&payload))))
}

async fn manga(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryRandomMangaCommand::parse(&query)?;
    let payload = random(&state, EntityKind::Manga, command.sfw, command.unapproved).await?;
    Ok(json_ok(kuukan_core::envelope::data(manga::manga(&payload))))
}

async fn characters(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryRandomCharacterCommand::parse(&query)?;
    let payload = random(&state, EntityKind::Character, false, false).await?;
    Ok(json_ok(kuukan_core::envelope::data(character::character(
        &payload,
    ))))
}

async fn people(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryRandomPersonCommand::parse(&query)?;
    let payload = random(&state, EntityKind::Person, false, false).await?;
    Ok(json_ok(kuukan_core::envelope::data(person::person(&payload))))
}

async fn users(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryRandomUserCommand::parse(&query)?;
    let payload = random(&state, EntityKind::User, false, false).await?;
    Ok(json_ok(kuukan_core::envelope::data(resource::user_item(
        &payload,
    ))))
}

/// `JikanApiModel::scopeRandom()` via the store; an empty result is a 404.
async fn random(
    state: &AppState,
    kind: EntityKind,
    sfw: bool,
    unapproved: bool,
) -> Result<serde_json::Value, ApiErrorResponse> {
    let entity = state
        .store
        .random_entities(kind, 1, sfw, unapproved)
        .await
        .map_err(storage_error)?
        .into_iter()
        .next()
        .ok_or_else(|| ApiErrorResponse(ApiError::not_found()))?;
    Ok(entity.payload)
}

/// Same rendering as `services::scrape` for storage failures.
fn storage_error(error: kuukan_store::StoreError) -> ApiErrorResponse {
    ApiErrorResponse(ApiError::Storage {
        error: Some(error.to_string()),
        report_url: None,
    })
}
