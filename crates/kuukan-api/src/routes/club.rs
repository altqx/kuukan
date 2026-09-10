//! `/clubs` routes — port of `ClubController`.
//!
//! Wiring mirrors the PHP handler/repository bindings
//! (`AppServiceProvider::requestHandlersWithScraperService`):
//!
//! - `main`, `staff` and `relations` extend `ItemLookupHandler` bound to the
//!   **clubs** repository, so they serve the stored club entity (scraping the
//!   full `ClubRequest` document on miss/expiry). `staff` and `relations`
//!   only read slices of that same document — they are *not* fingerprint
//!   caches;
//! - `members` extends `RequestHandlerWithScraperCache` bound to the
//!   `clubs_members` documents, i.e. a fingerprint-keyed cache of
//!   `getClubUsers` for the requested page.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::config::CacheCategory;
use crate::dto::club::{
    ClubLookupCommand, ClubMembersLookupCommand, ClubRelationLookupCommand, ClubStaffLookupCommand,
};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::club as resource;
use crate::services::scrape::{
    cache_or_scrape, entity_or_scrape, fingerprint, request_uri, CachedPayload,
};
use crate::state::AppState;
use kuukan_core::envelope;
use kuukan_store::EntityKind;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/clubs/{id}", get(main))
        .route("/clubs/{id}/members", get(members))
        .route("/clubs/{id}/staff", get(staff))
        .route("/clubs/{id}/relations", get(relations))
}

/// Shared entity lookup: validate, fetch the club entity (scrape on miss) and
/// compute the endpoint fingerprint/ttl.
async fn load_club(
    state: &AppState,
    uri: &axum::http::Uri,
    id: i64,
) -> Result<(CachedPayload, u64, String), ApiErrorResponse> {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);
    let mal = state.mal.clone();
    let cached = entity_or_scrape(state, EntityKind::Club, id, ttl, move || async move {
        kuukan_mal::api::club::get_club(&mal, id).await
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
    let command = ClubLookupCommand::parse(id, &query)?;
    let (cached, ttl, uri) = load_club(&state, &uri, command.id).await?;
    let data = envelope::data(resource::club(&cached.payload));
    Ok(json_with_cache_flags(
        data,
        &fingerprint("clubs", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn members(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubMembersLookupCommand::parse(id, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);
    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "clubs", &uri, ttl, move || async move {
        kuukan_mal::api::club::get_club_users(&mal, command.id, command.page).await
    })
    .await?;
    let data = resource::club_members(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("clubs", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn staff(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubStaffLookupCommand::parse(id, &query)?;
    let (cached, ttl, uri) = load_club(&state, &uri, command.id).await?;
    let data = envelope::data(resource::club_staff(&cached.payload));
    Ok(json_with_cache_flags(
        data,
        &fingerprint("clubs", &uri),
        cached.modified_at,
        ttl,
    ))
}

async fn relations(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubRelationLookupCommand::parse(id, &query)?;
    let (cached, ttl, uri) = load_club(&state, &uri, command.id).await?;
    let data = envelope::data(resource::club_relations(&cached.payload));
    Ok(json_with_cache_flags(
        data,
        &fingerprint("clubs", &uri),
        cached.modified_at,
        ttl,
    ))
}
