//! Top routes — `/top/anime`, `/top/manga`, `/top/characters`, `/top/people`,
//! `/top/reviews`.
//!
//! Port of `TopController` + `app/Features/QueryTop*Handler.php`.
//!
//! The four item lists are Mongo queries in PHP (`Default*Repository`), so both
//! the ordering and the residual filters are reproduced on the search index:
//!
//! | endpoint | default order | `filter` alternatives |
//! |---|---|---|
//! | `/top/anime` | `score desc` | `airing`: score desc + airing; `upcoming`: members desc + status; `bypopularity`: members desc; `favorite`: favorites desc |
//! | `/top/manga` | `score desc` | `publishing`: members desc + publishing; `upcoming`: members desc + status; `bypopularity`: members desc; `favorite`: favorites desc |
//! | `/top/characters` | `member_favorites desc` | — |
//! | `/top/people` | `member_favorites desc` | — |
//!
//! `airing`/`publishing` are booleans in Mongo; the index carries the derived
//! `status` instead ("Currently Airing"/"Publishing"), the same set of MAL
//! entries. Top commands have no `unapproved` parameter, so, like PHP, the
//! approved-only clause is skipped.
//!
//! `/top/reviews` is the `RequestHandlerWithScraperCache` endpoint: it reads and
//! writes the fingerprint-keyed reviews document and renders it through
//! `ResultsResource` with the `CacheCategory::Default` flags (recorded
//! reference: `X-Request-Fingerprint: request:top:...`,
//! `Cache-Control: public, s-maxage=86400`).

use axum::extract::{OriginalUri, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use kuukan_core::enums::{TopAnimeFilter, TopMangaFilter};
use kuukan_core::error::ApiError;
use kuukan_search::{EntityKind, SearchParams, SortDirection as SearchSort};

use crate::config::CacheCategory;
use crate::dto::top::{
    QueryTopAnimeItemsCommand, QueryTopCharactersCommand, QueryTopMangaItemsCommand,
    QueryTopPeopleCommand, QueryTopReviewsCommand,
};
use crate::error::{json_ok, ApiErrorResponse};
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::top as resource;
use crate::services::scrape::{cache_or_scrape, fingerprint, request_uri};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/top/anime", get(anime))
        .route("/top/manga", get(manga))
        .route("/top/characters", get(characters))
        .route("/top/people", get(people))
        .route("/top/reviews", get(reviews))
        .layer(axum::middleware::from_fn(super::search::no_cache_default))
}

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

async fn anime(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryTopAnimeItemsCommand::parse(&query)?;
    let (order_by, status) = match command.filter {
        None => ("score", None),
        Some(filter) => match filter {
            TopAnimeFilter::Airing => ("score", Some("Currently Airing")),
            TopAnimeFilter::Upcoming => ("members", Some("Not yet aired")),
            TopAnimeFilter::ByPopularity => ("members", None),
            TopAnimeFilter::Favorite => ("favorites", None),
        },
    };

    let params = SearchParams {
        page: Some(command.page),
        limit: Some(command.limit),
        order_by: Some(order_by.to_string()),
        sort: Some(SearchSort::Desc),
        status: status.map(str::to_string),
        media_type: command.r#type.map(|value| value.as_str().to_string()),
        rating: command.rating.map(|value| value.as_str().to_string()),
        sfw: Some(command.sfw),
        // Top commands have no `unapproved` parameter: PHP never adds the
        // approved-only filter here.
        unapproved: Some(true),
        ..SearchParams::default()
    };
    let result = search(&state, EntityKind::Anime, &params)?;
    // `AnimeResource` reads the Eloquent accessors (`season`, `year`,
    // `broadcast`), which the index keeps in their raw JMS shape.
    let items: Vec<serde_json::Value> = result
        .items
        .iter()
        .map(crate::routes::season::materialize_accessors)
        .collect();
    Ok(json_ok(resource::top_anime(
        &super::search::search_page(&result),
        &items,
    )))
}

async fn manga(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryTopMangaItemsCommand::parse(&query)?;
    let (order_by, status) = match command.filter {
        None => ("score", None),
        Some(filter) => match filter {
            TopMangaFilter::Publishing => ("members", Some("Publishing")),
            TopMangaFilter::Upcoming => ("members", Some("Not yet published")),
            TopMangaFilter::ByPopularity => ("members", None),
            TopMangaFilter::Favorite => ("favorites", None),
        },
    };

    let params = SearchParams {
        page: Some(command.page),
        limit: Some(command.limit),
        order_by: Some(order_by.to_string()),
        sort: Some(SearchSort::Desc),
        status: status.map(str::to_string),
        media_type: command.r#type.map(|value| value.as_str().to_string()),
        sfw: Some(command.sfw),
        unapproved: Some(true),
        ..SearchParams::default()
    };
    let result = search(&state, EntityKind::Manga, &params)?;
    Ok(json_ok(resource::top_manga(
        &super::search::search_page(&result),
        &result.items,
    )))
}

async fn characters(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryTopCharactersCommand::parse(&query)?;
    let params = SearchParams {
        page: Some(command.page),
        limit: Some(command.limit),
        order_by: Some("member_favorites".to_string()),
        sort: Some(SearchSort::Desc),
        unapproved: Some(true),
        ..SearchParams::default()
    };
    let result = search(&state, EntityKind::Character, &params)?;
    Ok(json_ok(resource::top_characters(
        &super::search::search_page(&result),
        &result.items,
    )))
}

async fn people(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryTopPeopleCommand::parse(&query)?;
    let params = SearchParams {
        page: Some(command.page),
        limit: Some(command.limit),
        order_by: Some("member_favorites".to_string()),
        sort: Some(SearchSort::Desc),
        unapproved: Some(true),
        ..SearchParams::default()
    };
    let result = search(&state, EntityKind::Person, &params)?;
    Ok(json_ok(resource::top_people(
        &super::search::search_page(&result),
        &result.items,
    )))
}

async fn reviews(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryTopReviewsCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);

    let r#type = command
        .r#type
        .map(|value| value.index())
        .unwrap_or("anime")
        .to_string();
    let spoilers = command.spoilers.unwrap_or(true);
    let preliminary = command.preliminary.unwrap_or(true);
    let page = command.page;

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "top", &uri, ttl, move || async move {
        // Port of `QueryTopReviewsHandler`: the handler calls
        // `new ReviewsRequest($type, $page, $spoilers, $preliminary)`, whose
        // third parameter is `$sort` — so the boolean spoilers flag lands in
        // the MAL `sort` query parameter (true -> "1"), the preliminary flag
        // controls `spoiler`, and `preliminary` stays at its `true` default.
        // Port the oddity: the fingerprints and cached documents must match.
        let sort = if spoilers { "1" } else { "" };
        kuukan_mal::api::reviews::get_reviews(&mal, &r#type, Some(page), sort, preliminary, true)
            .await
    })
    .await?;

    Ok(json_with_cache_flags(
        resource::top_reviews(&cached.payload),
        &fingerprint("top", &uri),
        cached.modified_at,
        ttl,
    ))
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Run the index query, mapping search failures to the Jikan 500.
fn search(
    state: &AppState,
    kind: EntityKind,
    params: &SearchParams,
) -> Result<kuukan_search::SearchResult, ApiErrorResponse> {
    state
        .search_index()
        .search(kind, params)
        .map_err(|error| ApiErrorResponse(ApiError::internal(error.to_string())))
}
