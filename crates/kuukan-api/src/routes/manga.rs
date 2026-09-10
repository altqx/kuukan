//! `/manga` routes — port of `MangaController` (`app/Features/Manga*Handler.php`).
//!
//! Two storage shapes are used:
//!
//! * **entity lookups** (`ItemLookupHandler`) serve the manga document stored
//!   under `(EntityKind::Manga, mal_id)`: `main`, `full`, `relations` and
//!   `external`;
//! * **fingerprint caches** (`RequestHandlerWithScraperCache`) scrape and cache
//!   the sub-resource under the request fingerprint: everything else.
//!
//! Single-object resources are wrapped with `envelope::data`; the list
//! resources (`news`, `userupdates`, `reviews`) already return the
//! `{pagination, data}` envelope.

use std::future::Future;

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use kuukan_core::envelope;
use kuukan_core::error::ApiError;
use kuukan_mal::error::MalError;
use kuukan_store::EntityKind;
use serde_json::{json, Value};

use crate::config::CacheCategory;
use crate::dto::manga::{
    MangaCharactersLookupCommand, MangaExternalLookupCommand, MangaForumLookupCommand,
    MangaFullLookupCommand, MangaLookupCommand, MangaMoreInfoLookupCommand,
    MangaNewsLookupCommand, MangaPicturesLookupCommand, MangaRecommendationsLookupCommand,
    MangaRelationsLookupCommand, MangaReviewsLookupCommand, MangaStatsLookupCommand,
    MangaUserUpdatesLookupCommand,
};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::manga as resource;
use crate::resources::misc;
use crate::services::scrape::{
    cache_or_scrape, entity_or_scrape, fingerprint, request_uri, CachedPayload,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/manga/{id}", get(main))
        .route("/manga/{id}/full", get(full))
        .route("/manga/{id}/characters", get(characters))
        .route("/manga/{id}/news", get(news))
        .route("/manga/{id}/forum", get(forum))
        .route("/manga/{id}/pictures", get(pictures))
        .route("/manga/{id}/statistics", get(statistics))
        .route("/manga/{id}/moreinfo", get(more_info))
        .route("/manga/{id}/recommendations", get(recommendations))
        .route("/manga/{id}/userupdates", get(user_updates))
        .route("/manga/{id}/reviews", get(reviews))
        .route("/manga/{id}/relations", get(relations))
        .route("/manga/{id}/external", get(external))
}

/// Laravel route constraints are `[0-9]+`: a non-numeric id never matches the
/// route and renders as `HttpException` 404, not axum's `Path<i64>` 400. Digit
/// strings that overflow `i64` saturate like PHP's numeric string cast.
fn route_id(value: &str) -> Result<i64, ApiErrorResponse> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiErrorResponse(ApiError::not_found()));
    }
    Ok(value.parse::<i64>().unwrap_or(i64::MAX))
}

/// Entity lookup (`ItemLookupHandler::handle` + `CachedScraperService::find`).
async fn load_entity(
    state: &AppState,
    uri: &axum::http::Uri,
    id: i64,
) -> Result<(CachedPayload, u64, String), ApiErrorResponse> {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);
    let mal = state.mal.clone();
    let cached = entity_or_scrape(state, EntityKind::Manga, id, ttl, move || async move {
        kuukan_mal::api::manga::get_manga(&mal, id).await
    })
    .await?;
    Ok((cached, ttl, uri))
}

/// Fingerprint-cache lookup
/// (`RequestHandlerWithScraperCache::handle` + `findList`).
async fn load_cache<F, Fut>(
    state: &AppState,
    uri: &axum::http::Uri,
    fetch: F,
) -> Result<(CachedPayload, u64, String), ApiErrorResponse>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Value, MalError>>,
{
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);
    let cached = cache_or_scrape(state, "manga", &uri, ttl, fetch).await?;
    Ok((cached, ttl, uri))
}

/// Render a mapper result with the Jikan cache flags for `uri`.
fn render(data: Value, uri: &str, cached: CachedPayload, ttl: u64) -> Response {
    json_with_cache_flags(
        data,
        &fingerprint("manga", uri),
        cached.modified_at,
        ttl,
    )
}

async fn main(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaLookupCommand::parse(route_id(&id)?, &query)?;
    let (cached, ttl, uri) = load_entity(&state, &uri, command.id).await?;
    let data = envelope::data(resource::manga(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn full(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaFullLookupCommand::parse(route_id(&id)?, &query)?;
    let (cached, ttl, uri) = load_entity(&state, &uri, command.id).await?;
    let data = envelope::data(resource::manga_full(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn characters(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaCharactersLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        let characters = kuukan_mal::api::manga::get_manga_characters(&mal, command.id).await?;
        Ok(json!({ "characters": characters }))
    })
    .await?;
    let data = envelope::data(resource::manga_characters(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn news(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaNewsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let page = command.page;
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        kuukan_mal::api::manga::get_manga_news(&mal, command.id, Some(page)).await
    })
    .await?;
    let data = resource::manga_news(&cached.payload);
    Ok(render(data, &uri, cached, ttl))
}

async fn forum(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaForumLookupCommand::parse(route_id(&id)?, &query)?;
    // `MangaForumLookupHandler` passes the filter through untouched (`null`
    // stays `null`; `MangaForumRequest` ignores values outside its valid set).
    let topic = command.filter.as_ref().map(|filter| filter.as_str());
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        kuukan_mal::api::manga::get_manga_forum(&mal, command.id, topic).await
    })
    .await?;
    let data = envelope::data(resource::manga_forum(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn pictures(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaPicturesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        let pictures = kuukan_mal::api::manga::get_manga_pictures(&mal, command.id).await?;
        Ok(json!({ "pictures": pictures }))
    })
    .await?;
    let data = envelope::data(resource::manga_pictures(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn statistics(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaStatsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        kuukan_mal::api::manga::get_manga_stats(&mal, command.id).await
    })
    .await?;
    let data = envelope::data(resource::manga_statistics(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn more_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaMoreInfoLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        kuukan_mal::api::manga::get_manga_more_info(&mal, command.id).await
    })
    .await?;
    let data = envelope::data(resource::manga_more_info(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaRecommendationsLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        let items = kuukan_mal::api::manga::get_manga_recommendations(&mal, command.id).await?;
        Ok(json!({ "recommendations": items }))
    })
    .await?;
    let data = envelope::data(resource::manga_recommendations(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn user_updates(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaUserUpdatesLookupCommand::parse(route_id(&id)?, &query)?;
    let mal = state.mal.clone();
    let page = command.page;
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        kuukan_mal::api::manga::get_manga_recently_updated_by_users(
            &mal,
            command.id,
            Some(page),
        )
        .await
    })
    .await?;
    // `MangaUserUpdatesLookupHandler` keeps the default `ResultsResource`.
    let data = misc::results(&cached.payload);
    Ok(render(data, &uri, cached, ttl))
}

async fn reviews(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaReviewsLookupCommand::parse(route_id(&id)?, &query)?;
    let params = command.review_request_params();
    let mal = state.mal.clone();
    let (cached, ttl, uri) = load_cache(&state, &uri, move || async move {
        kuukan_mal::api::manga::get_manga_reviews(
            &mal,
            command.id,
            Some(params.page),
            Some(params.sort.as_str()),
            Some(params.spoilers),
            Some(params.preliminary),
        )
        .await
    })
    .await?;
    let data = resource::manga_reviews(&cached.payload);
    Ok(render(data, &uri, cached, ttl))
}

async fn relations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaRelationsLookupCommand::parse(route_id(&id)?, &query)?;
    let (cached, ttl, uri) = load_entity(&state, &uri, command.id).await?;
    let data = envelope::data(resource::manga_relations(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}

async fn external(
    State(state): State<AppState>,
    Path(id): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaExternalLookupCommand::parse(route_id(&id)?, &query)?;
    let (cached, ttl, uri) = load_entity(&state, &uri, command.id).await?;
    let data = envelope::data(resource::manga_external_links(&cached.payload));
    Ok(render(data, &uri, cached, ttl))
}
