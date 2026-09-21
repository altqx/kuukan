//! `/schedules` route — port of `ScheduleController@main` +
//! `QueryAnimeSchedulesHandler`.
//!
//! jikan-rest queries `DefaultAnimeRepository::getCurrentlyAiring()` (Mongo)
//! and applies the `sfw`/`kids`/`unapproved` scopes. kuukan has no query
//! builder, so the stored `anime` entities are filtered in memory with the
//! same predicates and paginated with the Laravel semantics.

use axum::extract::{OriginalUri, Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use kuukan_core::enums::AnimeScheduleFilter;
use kuukan_core::error::ApiError;
use serde_json::Value;

use crate::collection::{AnimeCollection, Members};
use crate::dto::schedule::QueryAnimeSchedulesCommand;
use crate::endpoint::Endpoint;
use crate::error::ApiErrorResponse;
use crate::extract::RawQuery;
use crate::resources::schedule as resource;
use crate::state::AppState;

/// What this module's endpoints are fingerprinted as.
const ENDPOINT: Endpoint = Endpoint::new("schedules");

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/schedules", get(main))
        .route("/schedules/{filter}", get(main_by_day))
}

async fn main(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    handle(&state, &uri, None, &query).await
}

/// Laravel route `schedules[/{filter:[A-Za-z]+}]`: a segment that is not pure
/// letters does not match the route (404), a letter segment reaches the DTO
/// (which validates the day enum).
async fn main_by_day(
    State(state): State<AppState>,
    Path(filter): Path<String>,
    OriginalUri(uri): OriginalUri,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiErrorResponse> {
    if filter.is_empty() || !filter.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(ApiError::not_found().into());
    }
    handle(&state, &uri, Some(&filter), &query).await
}

async fn handle(
    state: &AppState,
    uri: &axum::http::Uri,
    filter: Option<&str>,
    query: &kuukan_core::params::Query,
) -> Result<Response, ApiErrorResponse> {
    let command = QueryAnimeSchedulesCommand::parse(filter, query)?;

    // `getCurrentlyAiring`: type = TV, status = Currently Airing, optional
    // broadcast day filter, ordered by members ascending.
    let mut collection = AnimeCollection::load(state).await?.retain(|item| {
        item.get("type").and_then(Value::as_str) == Some("TV")
            && item.get("status").and_then(Value::as_str) == Some("Currently Airing")
    });
    if let Some(day) = command.filter {
        collection = collection.retain(|item| broadcast_matches(item, day));
    }
    let (pagination, page_items) = collection
        .media_filters(command.sfw, command.kids, command.unapproved)
        .order_by_members(Members::Ascending)
        .page(command.page, command.limit);
    let data = resource::schedules(&pagination, &page_items);

    // Repository responses carry no cached document: epoch headers.
    Ok(ENDPOINT.flags(state, uri, 0).render(data))
}

/// `getCurrentlyAiring` broadcast predicate:
///
/// * weekdays: `where("broadcast", "like", "<label>%")` — the label is the
///   PHP enum's lowercase day; stored documents are either the raw MAL string
///   (`"Mondays at 18:00 (JST)"`) or the adapted object (`{day: "Mondays",..}`);
/// * `other`/`unknown`: `where("broadcast", "<label>")`.
fn broadcast_matches(item: &Value, filter: AnimeScheduleFilter) -> bool {
    let broadcast = item.get("broadcast");
    let as_string = broadcast.and_then(Value::as_str);
    let as_object = broadcast.and_then(Value::as_object);
    let day = as_object
        .and_then(|object| object.get("day"))
        .and_then(Value::as_str)
        .or_else(|| {
            as_string
                .and_then(|value| value.split(" at ").next())
                .filter(|value| !value.is_empty())
        });

    if filter.is_week_day() {
        // PHP label is the lowercase index (e.g. `monday`).
        day.is_some_and(|day| day.to_ascii_lowercase().starts_with(filter.as_str()))
    } else {
        let label = filter.as_str();
        day == Some(label)
            || as_string == Some(label)
            || as_object
                .and_then(|object| object.get("string"))
                .and_then(Value::as_str)
                == Some(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn weekday_filter_matches_raw_string_and_adapted_object() {
        let raw = json!({"broadcast": "Mondays at 18:00 (JST)"});
        assert!(broadcast_matches(&raw, AnimeScheduleFilter::Monday));
        assert!(!broadcast_matches(&raw, AnimeScheduleFilter::Tuesday));

        let adapted = json!({"broadcast": {
            "day": "Mondays",
            "time": "18:00",
            "timezone": "Asia/Tokyo",
            "string": "Mondays at 18:00 (JST)",
        }});
        assert!(broadcast_matches(&adapted, AnimeScheduleFilter::Monday));
        assert!(!broadcast_matches(&adapted, AnimeScheduleFilter::Friday));
    }

    #[test]
    fn other_and_unknown_match_their_labels() {
        assert!(broadcast_matches(
            &json!({"broadcast": "Not scheduled once per week"}),
            AnimeScheduleFilter::Other
        ));
        assert!(broadcast_matches(
            &json!({"broadcast": "Unknown"}),
            AnimeScheduleFilter::Unknown
        ));
        assert!(!broadcast_matches(
            &json!({"broadcast": "Mondays at 18:00 (JST)"}),
            AnimeScheduleFilter::Unknown
        ));
    }
}
