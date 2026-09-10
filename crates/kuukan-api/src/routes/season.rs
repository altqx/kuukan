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
use kuukan_core::enums::{
    AnimeSeason, AnimeType, GENRE_ANIME_EROTICA, GENRE_ANIME_HENTAI, GENRE_ANIME_KIDS,
};
use kuukan_core::error::ApiError;
use kuukan_core::pagination::{paginate, Pagination};
use kuukan_store::EntityKind;
use serde_json::{json, Value};

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
    let mut items = load_anime(&state).await?;
    items.retain(|item| {
        item.get("status").and_then(Value::as_str) == Some("Not yet aired")
            && type_matches(item, command.filter)
            && passes_media_filters(item, command.sfw, command.kids, command.unapproved)
    });
    sort_by_members_desc(&mut items);
    Ok(render(
        &state,
        &uri,
        &command.season,
        items,
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
) -> Result<Vec<Value>, ApiErrorResponse> {
    let (from, to_end) = season_range(year, season);
    let premiered = season_label(year, season);
    let mut items = load_anime(state).await?;
    items.retain(|item| {
        season_matches(
            item,
            year,
            &premiered,
            &from,
            &to_end,
            command.filter,
            command.continuing,
        ) && passes_media_filters(item, command.sfw, command.kids, command.unapproved)
    });
    sort_by_members_desc(&mut items);
    Ok(items)
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
                if start <= *from && end.map_or(false, |end| end >= *from) {
                    matched = true;
                }
                // MAL has not published the end date yet; keep entries that
                // started within the last three months with enough episodes.
                let months = (from.year() - start.year()) * 12
                    + (from.month() as i32 - start.month() as i32);
                if end.is_none() && episodes.map_or(false, |e| e >= 14) && months > 0 && months <= 3
                {
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

/// Every stored `anime` entity, in `mal_id` order. The store has no query
/// builder, so repository list endpoints filter the loaded documents.
pub(crate) async fn load_anime(state: &AppState) -> Result<Vec<Value>, ApiErrorResponse> {
    let total = state
        .store
        .entity_count(EntityKind::Anime)
        .await
        .map_err(storage_error)?;
    let entities = state
        .store
        .list_entities(EntityKind::Anime, 0, total)
        .await
        .map_err(storage_error)?;
    Ok(entities.into_iter().map(|entity| entity.payload).collect())
}

pub(crate) fn storage_error(err: kuukan_store::StoreError) -> ApiErrorResponse {
    ApiErrorResponse(ApiError::Storage {
        error: Some(err.to_string()),
        report_url: None,
    })
}

/// `scopeFilter` (`App\Filters\FilterQueryString`) with `sfw`, `kids` and
/// `unapproved` — the only parameters the season/schedule DTOs expose that the
/// `Anime::$filters` allow-list actually applies.
pub(crate) fn passes_media_filters(
    item: &Value,
    sfw: bool,
    kids: Option<bool>,
    unapproved: bool,
) -> bool {
    // filterByUnapproved: `!$value ? where("approved", true) : $query`.
    if !unapproved && item.get("approved").and_then(Value::as_bool) != Some(true) {
        return false;
    }

    // filterBySfw -> scopeExceptItemsWithAdultRating.
    if sfw {
        if item.get("rating").and_then(Value::as_str) == Some("Rx - Hentai") {
            return false;
        }
        if has_mal_id(item, "demographics", GENRE_ANIME_HENTAI)
            || has_mal_id(item, "demographics", GENRE_ANIME_EROTICA)
            || has_mal_id(item, "genres", GENRE_ANIME_HENTAI)
        {
            return false;
        }
    }

    // filterByKids -> scopeOnlyKidsItems / scopeExceptKidsItems.
    if let Some(kids) = kids {
        if has_mal_id(item, "demographics", GENRE_ANIME_KIDS) != kids {
            return false;
        }
    }

    true
}

fn has_mal_id(item: &Value, field: &str, mal_id: i32) -> bool {
    item.get(field)
        .and_then(Value::as_array)
        .map_or(false, |entries| {
            entries
                .iter()
                .any(|entry| entry.get("mal_id").and_then(Value::as_i64) == Some(i64::from(mal_id)))
        })
}

pub(crate) fn members_of(item: &Value) -> f64 {
    item.get("members")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NEG_INFINITY)
}

/// `orderBy("members", "desc")`; missing members sort like BSON null (last).
pub(crate) fn sort_by_members_desc(items: &mut [Value]) {
    items.sort_by(|a, b| members_of(b).total_cmp(&members_of(a)));
}

/// `orderBy("members")` (ascending); missing members sort like BSON null (first).
pub(crate) fn sort_by_members_asc(items: &mut [Value]) {
    items.sort_by(|a, b| members_of(a).total_cmp(&members_of(b)));
}

/// The Laravel paginator's page window (`LengthAwarePaginator::items()`).
pub(crate) fn page_slice(items: Vec<Value>, page: u64, per_page: u64) -> Vec<Value> {
    let start = page.saturating_sub(1).saturating_mul(per_page);
    items
        .into_iter()
        .skip(start as usize)
        .take(per_page as usize)
        .collect()
}

/// Render one `AnimeCollection` page. Repository responses carry no cached
/// document, so `Last-Modified` is the epoch (PHP `CachedData` from the
/// paginator collection).
fn render(
    state: &AppState,
    uri: &axum::http::Uri,
    command: &QueryAnimeSeasonCommand,
    items: Vec<Value>,
    map: fn(&Pagination, &[Value]) -> Value,
) -> Response {
    let ttl = state.config.cache_ttl(CacheCategory::Default);
    let uri = request_uri(uri);
    let pagination = paginate(items.len() as u64, command.page, command.limit);
    let page_items: Vec<Value> = page_slice(items, command.page, command.limit)
        .iter()
        .map(materialize_accessors)
        .collect();
    let data = map(&pagination, &page_items);
    json_with_cache_flags(data, &fingerprint("seasons", &uri), 0, ttl)
}

// ---------------------------------------------------------------------------
// Eloquent accessors (`App\Anime` getters)
// ---------------------------------------------------------------------------

/// Apply the `Anime` model accessors the PHP resource reads: `season`, `year`
/// (derived from `premiered`) and the adapted `broadcast` object. The store
/// keeps the JMS payload, so the route materializes them before
/// `AnimeResource`; the conversion is idempotent.
pub(crate) fn materialize_accessors(item: &Value) -> Value {
    let mut out = item.clone();
    let Some(object) = out.as_object_mut() else {
        return out;
    };

    let (season, year) = season_and_year(object.get("premiered").and_then(Value::as_str));
    object.insert("season".to_string(), season);
    object.insert("year".to_string(), year);

    let broadcast = adapt_broadcast(object.get("broadcast"));
    object.insert("broadcast".to_string(), broadcast);
    out
}

/// `getSeasonAttribute` / `getYearAttribute`, including the unanchored
/// `~(Winter|Spring|Summer|Fall|)\s([\d+]{4})~` check and the `(int)` cast.
fn season_and_year(premiered: Option<&str>) -> (Value, Value) {
    let Some(premiered) = premiered.filter(|value| !value.is_empty()) else {
        return (Value::Null, Value::Null);
    };
    if !has_season_year(premiered) {
        return (Value::Null, Value::Null);
    }
    let mut parts = premiered.split(' ');
    let season = parts.next().unwrap_or_default().to_lowercase();
    let year = php_int_cast(parts.next().unwrap_or_default());
    (Value::String(season), json!(year))
}

/// `preg_match('~(Winter|Spring|Summer|Fall|)\s([\d+]{4})~', $premiered)`: the
/// empty alternative means any ASCII whitespace followed by four characters
/// from `[\d+]` matches.
fn has_season_year(premiered: &str) -> bool {
    let bytes = premiered.as_bytes();
    (0..bytes.len().saturating_sub(1)).any(|index| {
        bytes[index].is_ascii_whitespace()
            && bytes.len() >= index + 5
            && bytes[index + 1..index + 5]
                .iter()
                .all(|byte| byte.is_ascii_digit() || *byte == b'+')
    })
}

/// PHP `(int) $value`: leading whitespace and an optional sign, then digits.
fn php_int_cast(value: &str) -> i64 {
    let trimmed = value.trim_start();
    let (sign, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (-1_i64, rest),
        None => (1_i64, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    let digits: String = digits.chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse::<i64>()
        .map(|number| number * sign)
        .unwrap_or(0)
}

/// `getBroadcastAttribute` -> `adaptBroadcastValue`.
fn adapt_broadcast(value: Option<&Value>) -> Value {
    match value {
        None | Some(Value::Null) => {
            json!({"day": null, "time": null, "timezone": null, "string": null})
        }
        Some(Value::Object(_)) => value.expect("matched object").clone(),
        Some(Value::String(broadcast)) => match split_broadcast(broadcast) {
            Some((day, time)) => json!({
                "day": day,
                "time": time,
                "timezone": "Asia/Tokyo",
                "string": broadcast,
            }),
            None => json!({
                "day": null,
                "time": null,
                "timezone": null,
                "string": broadcast,
            }),
        },
        Some(other) => other.clone(),
    }
}

/// `preg_match('~(.*) at (.*) \(~', $broadcast)`: greedy groups, so the last
/// `" at "` before the last `" ("` wins.
fn split_broadcast(broadcast: &str) -> Option<(String, String)> {
    let close = broadcast.rfind(" (")?;
    let head = &broadcast[..close];
    let at = head.rfind(" at ")?;
    Some((
        broadcast[..at].to_string(),
        broadcast[at + " at ".len()..close].to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn accessors_derive_season_year_and_broadcast() {
        let mapped = materialize_accessors(&item(json!({
            "mal_id": 2,
            "premiered": "Summer 2026",
            "broadcast": "Tuesdays at 23:00 (JST)",
            "aired": {"from": "2026-07-01T00:00:00+00:00", "to": null, "string": "Jul, 2026 to ?"},
        })));
        assert_eq!(mapped["season"], json!("summer"));
        assert_eq!(mapped["year"], json!(2026));
        assert_eq!(
            mapped["broadcast"],
            json!({"day": "Tuesdays", "time": "23:00", "timezone": "Asia/Tokyo", "string": "Tuesdays at 23:00 (JST)"})
        );

        // No premiered -> null season/year; null broadcast -> null object.
        let mapped = materialize_accessors(&item(json!({"mal_id": 3})));
        assert_eq!(mapped["season"], Value::Null);
        assert_eq!(mapped["year"], Value::Null);
        assert_eq!(
            mapped["broadcast"],
            json!({"day": null, "time": null, "timezone": null, "string": null})
        );

        // A broadcast without " at " keeps the string but nulls the parts.
        let mapped =
            materialize_accessors(&item(json!({"broadcast": "Not scheduled once per week"})));
        assert_eq!(
            mapped["broadcast"],
            json!({"day": null, "time": null, "timezone": null, "string": "Not scheduled once per week"})
        );

        // Idempotent for an already adapted document.
        let adapted = mapped.clone();
        assert_eq!(materialize_accessors(&adapted), adapted);
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

    #[test]
    fn media_filters_match_the_scopes() {
        let normal = item(json!({
            "approved": true,
            "rating": "PG-13 - Teens 13 or older",
            "genres": [{"mal_id": 1}],
            "demographics": [{"mal_id": 27}],
        }));
        assert!(passes_media_filters(&normal, true, None, false));
        assert!(!passes_media_filters(
            &json!({"approved": false}),
            false,
            None,
            false
        ));
        assert!(passes_media_filters(
            &json!({"approved": false}),
            false,
            None,
            true
        ));

        let hentai = item(json!({
            "approved": true,
            "rating": "Rx - Hentai",
            "genres": [{"mal_id": 12}],
            "demographics": [],
        }));
        assert!(!passes_media_filters(&hentai, true, None, false));
        assert!(passes_media_filters(&hentai, false, None, false));

        let kids = item(json!({
            "approved": true,
            "genres": [],
            "demographics": [{"mal_id": 15}],
        }));
        assert!(passes_media_filters(&kids, false, Some(true), false));
        assert!(!passes_media_filters(&kids, false, Some(false), false));
        assert!(!passes_media_filters(&normal, false, Some(true), false));
        assert!(passes_media_filters(&normal, false, Some(false), false));
    }

    #[test]
    fn pagination_window_is_laravel_like() {
        let items: Vec<Value> = (0..5).map(|id| json!({"mal_id": id})).collect();
        assert_eq!(page_slice(items.clone(), 1, 2).len(), 2);
        assert_eq!(page_slice(items.clone(), 3, 2).len(), 1);
        assert!(page_slice(items, 4, 2).is_empty());
    }

    #[test]
    fn sort_orders_missing_members_like_bson_null() {
        let mut items = vec![
            json!({"mal_id": 1, "members": 100}),
            json!({"mal_id": 2}),
            json!({"mal_id": 3, "members": 50}),
        ];
        sort_by_members_desc(&mut items);
        assert_eq!(items[0]["mal_id"], json!(1));
        assert_eq!(items[1]["mal_id"], json!(3));
        assert_eq!(items[2]["mal_id"], json!(2));

        sort_by_members_asc(&mut items);
        assert_eq!(items[0]["mal_id"], json!(2));
        assert_eq!(items[1]["mal_id"], json!(3));
        assert_eq!(items[2]["mal_id"], json!(1));
    }
}
