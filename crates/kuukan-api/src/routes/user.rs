//! `/users` routes — port of `UserController` (profile family, history,
//! friends, user lists, recommendations, reviews, clubs, external, recently
//! online) and `SearchController@userById`.
//!
//! Every endpoint is served through the fingerprint-keyed scraper cache
//! (`cache_or_scrape`, `request:users:<sha1(uri)>`), exactly like the PHP
//! handlers that extend `RequestHandlerWithScraperCache`: the full request URI
//! (path **and** query string) is part of the fingerprint, so `?page=1` and
//! `?type=anime` produce distinct cache documents. The X-Request-Fingerprint
//! header and the storage key are both derived from that same URI.
//!
//! TTL categories (`config/jikan.php::per_endpoint_cache_ttl`):
//!
//! | Endpoints | Category |
//! |---|---|
//! | `profile`, `statistics`, `favorites`, `about`, `history`, `friends`, `recommendations`, `reviews`, `clubs` | `User` |
//! | `full`, `userupdates`, `external` | `Default` (not listed in `per_endpoint_cache_ttl`, so PHP falls back to `default_cache_expire`) |
//! | `animelist`, `mangalist` | `UserList` |
//! | `userbyid` | `Search` (`SearchController@userById`) |
//! | `recentlyonline` | `Default` (`UserController@recentlyOnline => CACHE_DEFAULT_EXPIRE`) |
//!
//! The recorded reference responses confirm these headers (`/users/neko/full`,
//! `/users/neko/userupdates` and `/users/neko/external` return
//! `s-maxage=86400`, the rest `s-maxage=300`/`3600`/`432000`).
//!
//! `DISABLE_USER_LISTS`: PHP conditionally registers `animelist`/`mangalist`
//! via `env('DISABLE_USER_LISTS') === false`. Kuukan builds its router at
//! startup and `disable_user_lists` is read for every request, so the routes
//! are always registered and the handlers return the exact Jikan
//! `HttpException` 404 body (`{"status":404,"type":"HttpException",
//! "message":"Not Found","error":null}`) when the flag is set — the closest
//! observable behavior to an unregistered route. If the flag is unset, the
//! lists stay enabled (kuukan default), matching `.env.dist`
//! (`DISABLE_USER_LISTS=false`).
//!
//! Upstream manga-list bug: `QueryMangaListOfUserHandler`
//! stores the scraped list under `"anime"`, while `UserProfileMangaListResource`
//! reads `"manga"`. The handler here stores `{"anime": [...]}` and the
//! resource mapper reads `"manga"`, so a fresh scrape renders
//! `{"data": []}`; a seeded document that carries `"manga"` (the reference
//! seed) renders the list.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::{json, Value};

use crate::config::CacheCategory;
use crate::dto::user::{
    QueryAnimeListOfUserCommand, QueryMangaListOfUserCommand, UserAboutLookupCommand,
    UserByIdLookupCommand, UserClubsLookupCommand, UserExternalLookupCommand,
    UserFavoritesLookupCommand, UserFriendsLookupCommand, UserFullLookupCommand,
    UserHistoryLookupCommand, UserProfileLookupCommand, UserRecommendationsLookupCommand,
    UserReviewsLookupCommand, UserStatisticsLookupCommand, UserUpdatesLookupCommand,
};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::{misc, user as resource};
use crate::services::scrape::{cache_or_scrape, fingerprint, request_uri};
use crate::state::AppState;
use kuukan_core::enums::{AnimeListStatus, MangaListStatus};
use kuukan_core::envelope;
use kuukan_core::error::ApiError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users/userbyid/{id}", get(user_by_id))
        .route("/users/recentlyonline", get(recently_online))
        .route("/users/{username}", get(profile))
        .route("/users/{username}/full", get(full))
        .route("/users/{username}/statistics", get(statistics))
        .route("/users/{username}/favorites", get(favorites))
        .route("/users/{username}/userupdates", get(user_updates))
        .route("/users/{username}/about", get(about))
        .route("/users/{username}/history", get(history))
        .route("/users/{username}/history/{kind}", get(history_type))
        .route("/users/{username}/friends", get(friends))
        .route("/users/{username}/animelist", get(animelist))
        .route(
            "/users/{username}/animelist/{status}",
            get(animelist_status),
        )
        .route("/users/{username}/mangalist", get(mangalist))
        .route(
            "/users/{username}/mangalist/{status}",
            get(mangalist_status),
        )
        .route("/users/{username}/recommendations", get(recommendations))
        .route("/users/{username}/reviews", get(reviews))
        .route("/users/{username}/clubs", get(clubs))
        .route("/users/{username}/external", get(external))
}

/// Shared lookup for every `/users/...` endpoint: scrape/cache the request URI
/// under the `users` fingerprint family and render `render(payload)` with the
/// Jikan cache flags.
async fn cached_user_response<F, Fut>(
    state: &AppState,
    uri: &axum::http::Uri,
    ttl: u64,
    fetch: F,
    render: impl FnOnce(&Value) -> Value,
) -> Result<Response, ApiErrorResponse>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, kuukan_mal::error::MalError>>,
{
    let uri = request_uri(uri);
    let cached = cache_or_scrape(state, "users", &uri, ttl, fetch).await?;
    Ok(json_with_cache_flags(
        render(&cached.payload),
        &fingerprint("users", &uri),
        cached.modified_at,
        ttl,
    ))
}

/// Profile-style lookup: PHP keys these by the user entity (username), so
/// query parameters (`?page=1&limit=3`) must not change the cache key. Only
/// paginated sub-resources keep the full-URI fingerprint.
async fn cached_profile_response<F, Fut>(
    state: &AppState,
    uri: &axum::http::Uri,
    ttl: u64,
    fetch: F,
    render: impl FnOnce(&Value) -> Value,
) -> Result<Response, ApiErrorResponse>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, kuukan_mal::error::MalError>>,
{
    let full_uri = request_uri(uri);
    // PHP looks these endpoints up in the user entity (keyed by username), so
    // every `/users/{name}/...` profile variant shares one cached document...
    let canonical = {
        let path = full_uri.split('?').next().unwrap_or(&full_uri);
        let mut segments = path.trim_start_matches('/').splitn(4, '/');
        match (segments.next(), segments.next(), segments.next()) {
            (Some(version), Some(family), Some(username)) => {
                format!("/{version}/{family}/{}", username.split('/').next().unwrap_or(username))
            }
            _ => path.to_string(),
        }
    };
    let cached = cache_or_scrape(state, "users", &canonical, ttl, fetch).await?;
    // ...while `X-Request-Fingerprint` still hashes the actual request URI.
    Ok(json_with_cache_flags(
        render(&cached.payload),
        &fingerprint("users", &full_uri),
        cached.modified_at,
        ttl,
    ))
}

/// `QueryAnimeListOfUserCommand::$status` -> MAL list status code.
///
/// The PHP enum index is the query value (`watching`) and its label is the MAL
/// code (`1`). Upstream, `JikanUserListRequestMapperService` hands the enum
/// object to `UserAnimeListRequest::__construct(int $status)`, which is a
/// `TypeError` (500) for any explicit status; the frozen `kuukan_mal` entry
/// point takes the numeric code, so the label is what gets wired.
fn anime_status_code(status: Option<AnimeListStatus>) -> Option<i64> {
    status.and_then(|status| status.as_str().parse().ok())
}

/// `QueryMangaListOfUserCommand::$status` -> MAL list status code.
fn manga_status_code(status: Option<MangaListStatus>) -> Option<i64> {
    status.and_then(|status| status.as_str().parse().ok())
}

/// `page` is `#[Numeric, Min(1)]` in PHP (unbounded); the frozen MAL entry
/// points take `u32`.
fn page_u32(page: u64) -> u32 {
    u32::try_from(page).unwrap_or(u32::MAX)
}

/// `DISABLE_USER_LISTS` guard shared by the two list endpoints.
fn ensure_user_lists_enabled(state: &AppState) -> Result<(), ApiErrorResponse> {
    if state.config.disable_user_lists {
        return Err(ApiErrorResponse(ApiError::not_found()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Search-ish user lookups
// ---------------------------------------------------------------------------

/// `SearchController@userById` (`GET /users/userbyid/{id}`) — cache TTL
/// `CACHE_SEARCH_EXPIRE`; the payload wraps `UserMetaBasic` under `results`.
async fn user_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserByIdLookupCommand::parse(id, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Search);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            async move {
                let user = kuukan_mal::api::user::get_username_by_id(&mal, command.id).await?;
                Ok(json!({ "results": user }))
            }
        },
        misc::results,
    )
    .await
}

/// `UserController@recentlyOnline` (`GET /users/recentlyonline`) — not listed
/// in `per_endpoint_cache_ttl`, so PHP uses `CACHE_DEFAULT_EXPIRE`.
async fn recently_online(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiErrorResponse> {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            async move {
                let users = kuukan_mal::api::user::get_recent_online_users(&mal).await?;
                Ok(json!({ "results": users }))
            }
        },
        misc::results,
    )
    .await
}

// ---------------------------------------------------------------------------
// Profile family (`users` collection, `findByKey("internal_username")`)
// ---------------------------------------------------------------------------

async fn profile(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserProfileLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(resource::profile(payload)),
    )
    .await
}

async fn full(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserFullLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(resource::profile_full(payload)),
    )
    .await
}

async fn statistics(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserStatisticsLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(resource::profile_statistics(payload)),
    )
    .await
}

async fn favorites(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserFavoritesLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(resource::profile_favorites(payload)),
    )
    .await
}

async fn user_updates(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserUpdatesLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(resource::profile_last_updates(payload)),
    )
    .await
}

async fn about(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserAboutLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(resource::profile_about(payload)),
    )
    .await
}

async fn external(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserExternalLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    cached_profile_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move { kuukan_mal::api::user::get_user_profile(&mal, &username).await }
        },
        |payload| envelope::data(misc::external_links(payload)),
    )
    .await
}

// ---------------------------------------------------------------------------
// History / friends / recommendations / reviews / clubs
// ---------------------------------------------------------------------------

async fn history(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserHistoryLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let kind = command.r#type.map(|kind| kind.as_str().to_string());
            async move {
                kuukan_mal::api::user::get_user_history(&mal, &username, kind.as_deref()).await
            }
        },
        |payload| envelope::data(resource::profile_history(payload)),
    )
    .await
}

async fn history_type(
    State(state): State<AppState>,
    Path((username, kind)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserHistoryLookupCommand::parse_with_type(&username, Some(&kind), &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let kind = command.r#type.map(|kind| kind.as_str().to_string());
            async move {
                kuukan_mal::api::user::get_user_history(&mal, &username, kind.as_deref()).await
            }
        },
        |payload| envelope::data(resource::profile_history(payload)),
    )
    .await
}

async fn friends(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserFriendsLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            async move { kuukan_mal::api::user::get_user_friends(&mal, &username, Some(page)).await }
        },
        misc::results,
    )
    .await
}

async fn recommendations(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserRecommendationsLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            async move {
                kuukan_mal::api::user::get_user_recommendations(&mal, &username, Some(page)).await
            }
        },
        misc::results,
    )
    .await
}

async fn reviews(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserReviewsLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            async move { kuukan_mal::api::user::get_user_reviews(&mal, &username, Some(page)).await }
        },
        misc::results,
    )
    .await
}

async fn clubs(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = UserClubsLookupCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::User);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            async move {
                let clubs = kuukan_mal::api::user::get_user_clubs(&mal, &username).await?;
                Ok(json!({ "results": clubs }))
            }
        },
        misc::results,
    )
    .await
}

// ---------------------------------------------------------------------------
// User lists (`DISABLE_USER_LISTS`)
// ---------------------------------------------------------------------------

async fn animelist(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    ensure_user_lists_enabled(&state)?;
    let command = QueryAnimeListOfUserCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::UserList);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            let status = anime_status_code(command.status);
            async move {
                let items =
                    kuukan_mal::api::user::get_user_anime_list(&mal, &username, Some(page), status)
                        .await?;
                Ok(json!({ "anime": items }))
            }
        },
        |payload| {
            let items = payload
                .get("anime")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            resource::user_profile_anime_list_response(&items)
        },
    )
    .await
}

async fn animelist_status(
    State(state): State<AppState>,
    Path((username, status)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    ensure_user_lists_enabled(&state)?;
    let command = QueryAnimeListOfUserCommand::parse_with_status(&username, Some(&status), &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::UserList);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            let status = anime_status_code(command.status);
            async move {
                let items =
                    kuukan_mal::api::user::get_user_anime_list(&mal, &username, Some(page), status)
                        .await?;
                Ok(json!({ "anime": items }))
            }
        },
        |payload| {
            let items = payload
                .get("anime")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            resource::user_profile_anime_list_response(&items)
        },
    )
    .await
}

async fn mangalist(
    State(state): State<AppState>,
    Path(username): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    ensure_user_lists_enabled(&state)?;
    let command = QueryMangaListOfUserCommand::parse(&username, &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::UserList);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            let status = manga_status_code(command.status);
            async move {
                let items =
                    kuukan_mal::api::user::get_user_manga_list(&mal, &username, Some(page), status)
                        .await?;
                // Upstream bug: the scrape is stored under `anime` while the
                // resource reads `manga` (upstream bug).
                Ok(json!({ "anime": items }))
            }
        },
        |payload| {
            let items = payload
                .get("manga")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            resource::user_profile_manga_list_response(&items)
        },
    )
    .await
}

async fn mangalist_status(
    State(state): State<AppState>,
    Path((username, status)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    ensure_user_lists_enabled(&state)?;
    let command = QueryMangaListOfUserCommand::parse_with_status(&username, Some(&status), &query)?;
    let ttl = state.config.cache_ttl(CacheCategory::UserList);
    cached_user_response(
        &state,
        &uri,
        ttl,
        || {
            let mal = state.mal.clone();
            let username = command.username.clone();
            let page = page_u32(command.page);
            let status = manga_status_code(command.status);
            async move {
                let items =
                    kuukan_mal::api::user::get_user_manga_list(&mal, &username, Some(page), status)
                        .await?;
                // Upstream bug: see `mangalist`.
                Ok(json!({ "anime": items }))
            }
        },
        |payload| {
            let items = payload
                .get("manga")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            resource::user_profile_manga_list_response(&items)
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes_follow_php_indexes() {
        assert_eq!(anime_status_code(Some(AnimeListStatus::Watching)), Some(1));
        assert_eq!(anime_status_code(Some(AnimeListStatus::All)), Some(7));
        assert_eq!(anime_status_code(None), None);
        assert_eq!(manga_status_code(Some(MangaListStatus::Completed)), Some(2));
        assert_eq!(manga_status_code(None), None);
    }

    #[test]
    fn page_conversion_clamps() {
        assert_eq!(page_u32(1), 1);
        assert_eq!(page_u32(u64::MAX), u32::MAX);
    }
}
