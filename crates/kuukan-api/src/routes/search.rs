//! Search routes — `/anime`, `/manga`, `/characters`, `/people`, `/users`,
//! `/producers`, `/clubs`, `/magazines`.
//!
//! Port of `SearchController` + `app/Features/*SearchHandler.php` onto kuukan's
//! Tantivy index. PHP resolves these requests through
//! `DefaultQueryBuilderService` (Typesense when `q` is set and no `letter`, the
//! Mongo/Eloquent query otherwise); kuukan always goes through the search index,
//! which reproduces both modes (`kuukan_search::query`).
//!
//! # Cache headers
//!
//! The Typesense-backed handlers return the `*Collection` resource straight
//! from the controller, so they never get `addJikanCacheFlags`; the reference
//! recordings confirm `Cache-Control: no-cache, private` (Symfony's default)
//! and no `X-Request-Fingerprint`. [`no_cache_default`] adds exactly that
//! default to any response of this family. `/users` is the exception: in PHP it
//! is a `RequestHandlerWithScraperCache` endpoint with the `CacheCategory::Search`
//! TTL, so it renders with `json_with_cache_flags`. Kuukan serves it from the
//! index, which has no `modifiedAt`, so `Last-Modified` is the current time
//! (the first-request PHP behavior).
//!
//! Index misses intentionally yield an empty page: Jikan's Typesense search
//! never falls back to scraping either.

use axum::extract::{OriginalUri, State};
use axum::http::header::CACHE_CONTROL;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use kuukan_core::enums::{Gender, SortDirection as ApiSort};
use kuukan_core::error::ApiError;
use kuukan_core::pagination::Pagination;
use kuukan_search::{EntityKind, SearchParams, SearchResult, SortDirection as SearchSort};
use serde_json::{json, Value};

use crate::config::CacheCategory;
use crate::dto::anime::AnimeSearchCommand;
use crate::dto::base::{MediaSearchCommand, SearchCommand};
use crate::dto::character::CharactersSearchCommand;
use crate::dto::club::ClubSearchCommand;
use crate::dto::magazine::MagazineSearchCommand;
use crate::dto::manga::MangaSearchCommand;
use crate::dto::person::PeopleSearchCommand;
use crate::dto::producer::ProducersSearchCommand;
use crate::dto::user::UsersSearchCommand;
use crate::error::{json_ok, ApiErrorResponse};
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::search as resource;
use crate::services::scrape::{fingerprint, request_uri};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/anime", get(anime))
        .route("/manga", get(manga))
        .route("/characters", get(characters))
        .route("/people", get(people))
        .route("/users", get(users))
        .route("/producers", get(producers))
        .route("/clubs", get(clubs))
        .route("/magazines", get(magazines))
        .layer(axum::middleware::from_fn(no_cache_default))
}

/// Symfony/Laravel default cache header for responses that never call
/// `addJikanCacheFlags` (`Response::__construct` calls `setCache` with these
/// directives; the recordings show them on every undecorated endpoint).
pub(crate) const NO_CACHE: &str = "no-cache, private";

/// Add Symfony's default `Cache-Control: no-cache, private` to every response
/// that did not set its own cache flags.
pub(crate) async fn no_cache_default(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    if !response.headers().contains_key(CACHE_CONTROL) {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(NO_CACHE));
    }
    response
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

async fn anime(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = AnimeSearchCommand::parse(&query)?;
    let result = search(&state, EntityKind::Anime, &anime_params(&command))?;
    // `AnimeResource` reads the Eloquent accessors (`season`, `year`,
    // `broadcast`), which the index keeps in their raw JMS shape.
    let items: Vec<Value> = result
        .items
        .iter()
        .map(crate::routes::season::materialize_accessors)
        .collect();
    Ok(json_ok(resource::anime_search(
        &search_page(&result),
        &items,
    )))
}

async fn manga(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MangaSearchCommand::parse(&query)?;
    let result = search(&state, EntityKind::Manga, &manga_params(&command))?;
    Ok(json_ok(resource::manga_search(
        &search_page(&result),
        &result.items,
    )))
}

async fn characters(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = CharactersSearchCommand::parse(&query)?;
    let params = SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        ..base_params(&command)
    };
    let result = search(&state, EntityKind::Character, &params)?;
    Ok(json_ok(resource::character_search(
        &search_page(&result),
        &result.items,
    )))
}

async fn people(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = PeopleSearchCommand::parse(&query)?;
    let params = SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        ..base_params(&command)
    };
    let result = search(&state, EntityKind::Person, &params)?;
    Ok(json_ok(resource::person_search(
        &search_page(&result),
        &result.items,
    )))
}

async fn users(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UsersSearchCommand::parse(&query)?;
    let params = SearchParams {
        q: command.q.clone(),
        page: Some(command.page),
        limit: Some(command.limit),
        sort: command.sort.map(search_sort),
        letter: command.letter.clone(),
        min_age: command.min_age.and_then(|age| u64::try_from(age).ok()),
        max_age: command.max_age.and_then(|age| u64::try_from(age).ok()),
        gender: command.gender.and_then(gender_value),
        location: command.location.clone(),
        ..SearchParams::default()
    };
    let result = search(&state, EntityKind::User, &params)?;
    let items: Vec<Value> = result.items.iter().map(user_search_item).collect();
    let data = kuukan_core::envelope::paged(&list_page(&result), items);

    let ttl = state.config.cache_ttl(CacheCategory::Search);
    let uri = request_uri(&uri);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("users", &uri),
        kuukan_store::now_unix(),
        ttl,
    ))
}

async fn producers(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ProducersSearchCommand::parse(&query)?;
    let params = SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        ..base_params(&command)
    };
    let result = search(&state, EntityKind::Producer, &params)?;
    Ok(json_ok(resource::producer_search(
        &search_page(&result),
        &result.items,
    )))
}

async fn clubs(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = ClubSearchCommand::parse(&query)?;
    let params = SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        media_type: command.r#type.map(|value| value.as_str().to_string()),
        category: command.category.map(|value| value.as_str().to_string()),
        ..base_params(&command)
    };
    let result = search(&state, EntityKind::Club, &params)?;
    Ok(json_ok(resource::club_search(
        &list_page(&result),
        &result.items,
    )))
}

async fn magazines(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = MagazineSearchCommand::parse(&query)?;
    let params = SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        ..base_params(&command)
    };
    let result = search(&state, EntityKind::Magazine, &params)?;
    Ok(json_ok(resource::magazine_search(
        &search_page(&result),
        &result.items,
    )))
}

// ---------------------------------------------------------------------------
// DTO -> SearchParams
// ---------------------------------------------------------------------------

/// `q`, `page`, `limit`, `sort` and `letter` shared by every search command.
fn base_params(search: &SearchCommand) -> SearchParams {
    SearchParams {
        q: search.q.clone(),
        page: Some(search.page),
        limit: Some(search.limit),
        order_by: None,
        sort: search.sort.map(search_sort),
        letter: search.letter.clone(),
        ..SearchParams::default()
    }
}

fn anime_params(command: &AnimeSearchCommand) -> SearchParams {
    SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        media_type: command.r#type.map(|value| value.as_str().to_string()),
        status: command.status.map(|value| value.as_str().to_string()),
        rating: command.rating.map(|value| value.as_str().to_string()),
        producer: command.producer.and_then(|id| u64::try_from(id).ok()),
        producers: command.producers.clone(),
        ..media_params(command)
    }
}

fn manga_params(command: &MangaSearchCommand) -> SearchParams {
    SearchParams {
        order_by: command.order_by.map(|value| value.as_str().to_string()),
        media_type: command.r#type.map(|value| value.as_str().to_string()),
        status: command.status.map(|value| value.as_str().to_string()),
        magazines: command.magazines.clone(),
        ..media_params(command)
    }
}

/// `MediaSearchCommand` fields (`sfw`, `unapproved`, scores, genres, dates).
fn media_params(command: &MediaSearchCommand) -> SearchParams {
    SearchParams {
        sfw: Some(command.sfw),
        unapproved: Some(command.unapproved),
        min_score: command.min_score,
        max_score: command.max_score,
        score: command.score,
        genres: command.genres.clone(),
        genres_exclude: command.genres_exclude.clone(),
        start_date: command.start_date.map(|date| date.to_ymd()),
        end_date: command.end_date.map(|date| date.to_ymd()),
        ..base_params(command)
    }
}

fn search_sort(sort: ApiSort) -> SearchSort {
    match sort {
        ApiSort::Asc => SearchSort::Asc,
        ApiSort::Desc => SearchSort::Desc,
    }
}

/// `/users` item shape. `UserSearchHandler` is a
/// `RequestHandlerWithScraperCache` endpoint whose `ResultsResource` passes the
/// MAL user-search hits through verbatim; a hit carries exactly
/// `{username, url, images, last_online}` (the recorded reference has no
/// `mal_id`/profile fields). Kuukan indexes full profiles, so project the same
/// four fields.
fn user_search_item(payload: &Value) -> Value {
    json!({
        "username": payload.get("username").cloned().unwrap_or(Value::Null),
        "url": payload.get("url").cloned().unwrap_or(Value::Null),
        "images": payload.get("images").cloned().unwrap_or(Value::Null),
        "last_online": payload.get("last_online").cloned().unwrap_or(Value::Null),
    })
}

/// `gender` as stored in the indexed profile (`ProfileResource`/MAL display
/// value). `any` means "no gender filter", matching PHP's `-1`.
fn gender_value(gender: Gender) -> Option<String> {
    match gender {
        Gender::Any => None,
        Gender::Male => Some("Male".to_string()),
        Gender::Female => Some("Female".to_string()),
        Gender::Nonbinary => Some("Non-Binary".to_string()),
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Run the index query, mapping storage failures to the Jikan 500.
fn search(
    state: &AppState,
    kind: EntityKind,
    params: &SearchParams,
) -> Result<SearchResult, ApiErrorResponse> {
    state
        .search_index()
        .search(kind, params)
        .map_err(|error| ApiErrorResponse(ApiError::internal(error.to_string())))
}

/// `pagination_plus` envelope data (`*Collection` resources); shared with
/// `routes::top`.
pub(crate) fn search_page(result: &SearchResult) -> Pagination {
    Pagination::search(
        result.last_page,
        result.current_page < result.last_page,
        result.current_page,
        result.items.len() as u64,
        result.total,
        result.per_page,
    )
}

/// Plain list pagination (`ClubCollection`, `ResultsResource`-over-search).
fn list_page(result: &SearchResult) -> Pagination {
    Pagination::list(result.last_page, result.current_page < result.last_page)
}
