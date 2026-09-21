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

use crate::dto::club::{
    ClubLookupCommand, ClubMembersLookupCommand, ClubRelationLookupCommand, ClubStaffLookupCommand,
};
use crate::endpoint::{Cached, Endpoint};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::club as resource;
use crate::state::AppState;

/// What this module's endpoints are cached and fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("clubs");
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
) -> Result<Cached, ApiErrorResponse> {
    let mal = state.mal.clone();
    ENDPOINT
        .entity(state, uri, EntityKind::Club, id, move || async move {
            kuukan_mal::api::club::get_club(&mal, id).await
        })
        .await
}

async fn main(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubLookupCommand::parse(id, &query)?;
    let cached = load_club(&state, &uri, command.id).await?;
    let data = envelope::data(resource::club(&cached.payload));
    Ok(cached.render(data))
}

async fn members(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubMembersLookupCommand::parse(id, &query)?;
    let mal = state.mal.clone();
    let cached = ENDPOINT
        .document(&state, &uri, move || async move {
            kuukan_mal::api::club::get_club_users(&mal, command.id, command.page).await
        })
        .await?;
    let data = resource::club_members(&cached.payload);
    Ok(cached.render(data))
}

async fn staff(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubStaffLookupCommand::parse(id, &query)?;
    let cached = load_club(&state, &uri, command.id).await?;
    let data = envelope::data(resource::club_staff(&cached.payload));
    Ok(cached.render(data))
}

async fn relations(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubRelationLookupCommand::parse(id, &query)?;
    let cached = load_club(&state, &uri, command.id).await?;
    let data = envelope::data(resource::club_relations(&cached.payload));
    Ok(cached.render(data))
}
