//! `/seasons` routes — port of `SeasonController` and its feature handlers.
//!
//! - `/seasons` (`QueryAnimeSeasonListHandler`) is a scraper-cache endpoint:
//!   the archive document is stored under the request fingerprint.
//! - `/seasons/now`, `/seasons/upcoming` and `/seasons/{year}/{season}`
//!   (`QueryCurrentAnimeSeasonHandler`, `QueryUpcomingAnimeSeasonHandler`,
//!   `QuerySpecificAnimeSeasonHandler`) query `DefaultAnimeRepository` in
//!   jikan-rest. kuukan has no Mongo query builder, so the `anime` entities of
//!   the store are filtered in memory with the exact repository predicates
//!   (see [`season_matches`] / [`passes_media_filters`]).
//!
//! The repository handlers never scrape: an empty store yields an empty
//! collection (`{"data": []}`), and the missing-document `CachedData` makes
//! `Last-Modified` the epoch, matching the recorded reference responses.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use kuukan_core::enums::{AnimeSeason, AnimeType};
use kuukan_core::error::ApiError;
use kuukan_core::pagination::Pagination;
use serde_json::Value;

use crate::collection::{AnimeCollection, Members};
use crate::config::CacheCategory;
use crate::dto::base::QueryAnimeSeasonCommand;
use crate::dto::seasonal::{
    QueryAnimeSeasonListCommand, QueryCurrentAnimeSeasonCommand, QuerySpecificAnimeSeasonCommand,
    QueryUpcomingAnimeSeasonCommand,
};
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::render::json_with_cache_flags;
use crate::resources::season as resource;
use crate::services::scrape::{cache_or_scrape, fingerprint, request_uri};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/seasons", get(archive))
        .route("/seasons/now", get(now))
        .route("/seasons/upcoming", get(upcoming))
        .route("/seasons/{year}/{season}", get(main))
}

/// `/seasons` — scraper cache (`QueryAnimeSeasonListHandler`).
async fn archive(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    QueryAnimeSeasonListCommand::parse(&query)?;
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(&uri);

    let mal = state.mal.clone();
    let cached = cache_or_scrape(&state, "seasons", &uri, ttl, move || async move {
        kuukan_mal::api::season_list::get_season_list(&mal).await
    })
    .await?;

    let data = resource::season_archive(&cached.payload);
    Ok(json_with_cache_flags(
        data,
        &fingerprint("seasons", &uri),
        cached.modified_at,
        ttl,
    ))
}

/// `/seasons/now` — current season in Asia/Tokyo (`QueryCurrentAnimeSeasonHandler`).
async fn now(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryCurrentAnimeSeasonCommand::parse(&query)?;
    let (year, season) = current_tokyo_season();
    let items = collect_season(&state, year, season, &command.season).await?;
    Ok(render(
        &state,
        &uri,
        &command.season,
        items,
        resource::season_now,
    ))
}

/// `/seasons/upcoming` — `getUpcomingSeasonItems` (`QueryUpcomingAnimeSeasonHandler`).
async fn upcoming(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryUpcomingAnimeSeasonCommand::parse(&query)?;
    let collection = AnimeCollection::load(&state)
        .await?
        .retain(|item| {
            item.get("status").and_then(Value::as_str) == Some("Not yet aired")
                && type_matches(item, command.filter)
        })
        .media_filters(command.sfw, command.kids, command.unapproved)
        .order_by_members(Members::Descending);
    Ok(render(
        &state,
        &uri,
        &command.season,
        collection,
        resource::season_upcoming,
    ))
}

/// `/seasons/{year}/{season}` (`QuerySpecificAnimeSeasonHandler`).
async fn main(
    State(state): State<AppState>,
    Path((year, season)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    // Laravel route: `{year:[0-9]{4}}/{season:[A-Za-z]+}`; anything else does
    // not match the route at all.
    if !is_four_digit_year(&year)
        || season.is_empty()
        || !season.bytes().all(|b| b.is_ascii_alphabetic())
    {
        return Err(ApiError::not_found().into());
    }
    let year_number: i64 = year.parse().expect("digits were checked");
    let command = QuerySpecificAnimeSeasonCommand::parse(year_number, &season, &query)?;
    let items = collect_season(
        &state,
        command.year as i32,
        command.season_name,
        &command.season,
    )
    .await?;
    Ok(render(
        &state,
        &uri,
        &command.season,
        items,
        resource::season_main,
    ))
}

// ---------------------------------------------------------------------------
// Repository query (`DefaultAnimeRepository`)
// ---------------------------------------------------------------------------

/// Shared `getItemsBySeason` query for `/seasons/now` and
/// `/seasons/{year}/{season}`: `season_items` + `scopeFilter` (`sfw`, `kids`,
/// `unapproved`) + `orderBy("members", "desc")`.
async fn collect_season(
    state: &AppState,
    year: i32,
    season: AnimeSeason,
    command: &QueryAnimeSeasonCommand,
) -> Result<AnimeCollection, ApiErrorResponse> {
    let (from, to_end) = season_range(year, season);
    let premiered = season_label(year, season);
    Ok(AnimeCollection::load(state)
        .await?
        .retain(|item| {
            season_matches(
                item,
                year,
                &premiered,
                &from,
                &to_end,
                command.filter,
                command.continuing,
            )
        })
        .media_filters(command.sfw, command.kids, command.unapproved)
        .order_by_members(Members::Descending))
}

/// Port of `DefaultAnimeRepository::getItemsBySeason`:
///
/// ```text
/// $or = [
///     ['premiered' => $premiered],
///     ['premiered' => null, 'aired.string' => ['$nin' => ["<year> to ?"]],
///      'aired.from' => ['$gte' => $from, '$lte' => $to->modify('last day of this month')]],
/// ]
/// ```
/// plus, when `$includeContinuingItems` is set, the three "continuing" OR
/// clauses (long-running shows, currently-airing carry overs and recent
/// premieres whose start date MAL has not exposed yet).
pub(crate) fn season_matches(
    item: &Value,
    year: i32,
    premiered: &str,
    from: &DateTime<Utc>,
    to_end: &DateTime<Utc>,
    type_filter: Option<AnimeType>,
    include_continuing: bool,
) -> bool {
    let premiered_field = item.get("premiered");
    let premiered_is_null = matches!(premiered_field, None | Some(Value::Null));
    let mut matched = premiered_field.and_then(Value::as_str) == Some(premiered);

    if !matched && premiered_is_null {
        let banned = format!("{year} to ?");
        let aired_string = item.pointer("/aired/string").and_then(Value::as_str);
        if aired_string != Some(banned.as_str()) {
            if let Some(start) = aired_datetime(item, "from") {
                if start >= *from && start <= *to_end {
                    matched = true;
                }
            }
        }
    }

    if !matched && include_continuing {
        let airing = item.get("airing").and_then(Value::as_bool) == Some(true);
        if airing {
            let start = aired_datetime(item, "from");
            let end = aired_datetime(item, "to");
            let episodes = item.get("episodes").and_then(Value::as_i64);
            if let Some(start) = start {
                // Long running shows.
                if start <= *from && end.is_none() && episodes.is_none() {
                    matched = true;
                }
                // Currently airing carry overs.
                if start <= *from && end.is_some_and(|end| end >= *from) {
                    matched = true;
                }
                // MAL has not published the end date yet; keep entries that
                // started within the last three months with enough episodes.
                let months = (from.year() - start.year()) * 12
                    + (from.month() as i32 - start.month() as i32);
                if end.is_none() && episodes.is_some_and(|e| e >= 14) && months > 0 && months <= 3 {
                    matched = true;
                }
            }
        }
    }

    matched && type_matches(item, type_filter)
}

fn type_matches(item: &Value, filter: Option<AnimeType>) -> bool {
    match filter {
        Some(filter) => item.get("type").and_then(Value::as_str) == Some(filter.as_str()),
        None => true,
    }
}

/// `generateSeasonRange`/`getSeasonRange`: first day of the season's first
/// month and last day of its last month, both at 00:00 UTC.
pub(crate) fn season_range(year: i32, season: AnimeSeason) -> (DateTime<Utc>, DateTime<Utc>) {
    let (start_month, end_month) = match season {
        AnimeSeason::Winter => (1, 3),
        AnimeSeason::Spring => (4, 6),
        AnimeSeason::Summer => (7, 9),
        AnimeSeason::Fall => (10, 12),
    };
    (month_start(year, start_month), month_end(year, end_month))
}

fn month_start(year: i32, month: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .expect("valid month start")
}

fn month_end(year: i32, month: u32) -> DateTime<Utc> {
    let next = if month == 12 {
        month_start(year + 1, 1)
    } else {
        month_start(year, month + 1)
    };
    next - Duration::days(1)
}

/// PHP `ucfirst($season) . " {$year}"` (`AnimeSeasonEnum` stringifies to its
/// lowercase index).
pub(crate) fn season_label(year: i32, season: AnimeSeason) -> String {
    let name = season.as_str();
    let mut chars = name.chars();
    let capitalized = match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    };
    format!("{capitalized} {year}")
}

/// Current season from `new DateTime('now', new DateTimeZone('Asia/Tokyo'))`.
/// Japan has no DST, so UTC+9 is exact.
pub(crate) fn current_tokyo_season() -> (i32, AnimeSeason) {
    let tokyo = Utc::now() + Duration::hours(9);
    let season = match tokyo.month() {
        1..=3 => AnimeSeason::Winter,
        4..=6 => AnimeSeason::Spring,
        7..=9 => AnimeSeason::Summer,
        _ => AnimeSeason::Fall,
    };
    (tokyo.year(), season)
}

fn is_four_digit_year(value: &str) -> bool {
    value.len() == 4 && value.bytes().all(|b| b.is_ascii_digit())
}

fn aired_datetime(item: &Value, side: &str) -> Option<DateTime<Utc>> {
    let raw = item.pointer(&format!("/aired/{side}"))?.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|moment| moment.with_timezone(&Utc))
}

// ---------------------------------------------------------------------------
// Shared list helpers (also used by `routes::schedule`)
// ---------------------------------------------------------------------------

/// Render one `AnimeCollection` page. Repository responses carry no cached
/// document, so `Last-Modified` is the epoch (PHP `CachedData` from the
/// paginator collection).
fn render(
    state: &AppState,
    uri: &axum::http::Uri,
    command: &QueryAnimeSeasonCommand,
    collection: AnimeCollection,
    map: fn(&Pagination, &[Value]) -> Value,
) -> Response {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);
    let (pagination, page_items) = collection.page(command.page, command.limit);
    let data = map(&pagination, &page_items);
    json_with_cache_flags(data, &fingerprint("seasons", &uri), 0, ttl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(properties: Value) -> Value {
        let mut map = properties.as_object().cloned().unwrap_or_default();
        map.entry("approved").or_insert(json!(true));
        Value::Object(map)
    }

    #[test]
    fn season_ranges_are_utc_month_bounds() {
        let (from, to) = season_range(2024, AnimeSeason::Winter);
        assert_eq!(from.to_rfc3339(), "2024-01-01T00:00:00+00:00");
        assert_eq!(to.to_rfc3339(), "2024-03-31T00:00:00+00:00");

        let (from, to) = season_range(2024, AnimeSeason::Fall);
        assert_eq!(from.to_rfc3339(), "2024-10-01T00:00:00+00:00");
        assert_eq!(to.to_rfc3339(), "2024-12-31T00:00:00+00:00");
    }

    #[test]
    fn season_labels_match_ucfirst() {
        assert_eq!(season_label(2024, AnimeSeason::Winter), "Winter 2024");
        assert_eq!(season_label(2026, AnimeSeason::Summer), "Summer 2026");
    }

    #[test]
    fn season_matches_ports_the_repository_or_clauses() {
        let (from, to_end) = season_range(2024, AnimeSeason::Winter);
        let matches = |properties: Value, continuing: bool| {
            season_matches(
                &item(properties),
                2024,
                "Winter 2024",
                &from,
                &to_end,
                None,
                continuing,
            )
        };

        // premiered match
        assert!(matches(json!({"premiered": "Winter 2024"}), false));
        // correct item without premiered: aired.from within the season
        assert!(matches(
            json!({
                "premiered": null,
                "airing": true,
                "aired": {"from": "2024-01-10T00:00:00+00:00", "to": "2024-02-15T00:00:00+00:00", "string": "Jan 10, 2024 to Feb 15, 2024"},
            }),
            false
        ));
        // garbled aired string is excluded
        assert!(!matches(
            json!({
                "premiered": null,
                "airing": false,
                "aired": {"from": "2024-01-01T00:00:00+00:00", "to": null, "string": "2024 to ?"},
            }),
            false
        ));
        // future airing date inside the season is kept
        assert!(matches(
            json!({
                "premiered": null,
                "airing": false,
                "aired": {"from": "2024-02-24T00:00:00+00:00", "to": null, "string": "Feb 24, 2024 to ?"},
            }),
            false
        ));
        // previous-season continuing item only with `continuing`
        let continuing = json!({
            "premiered": "Fall 2023",
            "airing": true,
            "aired": {"from": "2023-10-10T00:00:00+00:00", "to": null, "string": "Oct 10, 2023 to ?"},
        });
        assert!(!matches(continuing.clone(), false));
        assert!(matches(continuing, true));
        // newly started season is not a carry over of the previous one
        let started = json!({
            "premiered": "Fall 2024",
            "airing": true,
            "episodes": 16,
            "aired": {"from": "2024-10-02T00:00:00+00:00", "to": null, "string": "Oct 2, 2024 to ?"},
        });
        let (summer_from, summer_to) = season_range(2024, AnimeSeason::Summer);
        assert!(!season_matches(
            &item(started),
            2024,
            "Summer 2024",
            &summer_from,
            &summer_to,
            None,
            true,
        ));
    }
}
