//! Querying the stored anime collection.
//!
//! jikan-rest answers `/seasons/*`, `/schedules` and the repository-backed
//! listings from `DefaultAnimeRepository`. kuukan has no query builder, so
//! those endpoints load the stored `anime` entities and apply the repository's
//! predicates in memory: filter, order, paginate, then materialize the
//! Eloquent accessors the resources read.
//!
//! Those five steps used to live in `routes/season.rs` as eight `pub(crate)`
//! items, and `routes/schedule.rs` imported five of them in one `use` to
//! reassemble the same pipeline by hand. They are not about seasons; they are
//! about the collection. [`AnimeCollection`] owns them, and the order of the
//! steps with them.

use kuukan_core::enums::{GENRE_ANIME_EROTICA, GENRE_ANIME_HENTAI, GENRE_ANIME_KIDS};
use kuukan_core::error::ApiError;
use kuukan_core::pagination::{paginate, Pagination};
use kuukan_store::EntityKind;
use serde_json::{json, Value};

use crate::error::ApiErrorResponse;
use crate::state::AppState;

/// `orderBy("members", ...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Members {
    /// Most members first; missing members sort like BSON null (last).
    Descending,
    /// Fewest members first; missing members sort like BSON null (first).
    Ascending,
}

/// The stored `anime` documents, in `mal_id` order, narrowed step by step.
///
/// Build one with [`AnimeCollection::load`], narrow it, then take a page. The
/// pagination block counts what survived the filters, not what the page holds,
/// which is what `LengthAwarePaginator` reports.
pub struct AnimeCollection {
    items: Vec<Value>,
}

impl AnimeCollection {
    /// Every stored `anime` entity.
    pub async fn load(state: &AppState) -> Result<Self, ApiErrorResponse> {
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
        Ok(AnimeCollection {
            items: entities.into_iter().map(|entity| entity.payload).collect(),
        })
    }

    /// Build one from documents already in hand (used by tests).
    pub fn from_items(items: Vec<Value>) -> Self {
        AnimeCollection { items }
    }

    /// Keep the documents an endpoint-specific predicate accepts.
    #[must_use]
    pub fn retain(mut self, keep: impl FnMut(&Value) -> bool) -> Self {
        self.items.retain(keep);
        self
    }

    /// `scopeFilter` (`App\Filters\FilterQueryString`) with `sfw`, `kids` and
    /// `unapproved` — the only parameters the DTOs expose that the
    /// `Anime::$filters` allow-list actually applies.
    #[must_use]
    pub fn media_filters(self, sfw: bool, kids: Option<bool>, unapproved: bool) -> Self {
        self.retain(|item| passes_media_filters(item, sfw, kids, unapproved))
    }

    /// `orderBy("members", ...)`.
    #[must_use]
    pub fn order_by_members(mut self, order: Members) -> Self {
        match order {
            Members::Descending => self
                .items
                .sort_by(|a, b| members_of(b).total_cmp(&members_of(a))),
            Members::Ascending => self
                .items
                .sort_by(|a, b| members_of(a).total_cmp(&members_of(b))),
        }
        self
    }

    /// How many documents survived.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Take one page, with the pagination block for the whole collection.
    ///
    /// Page items carry the Eloquent accessors materialized, because that is
    /// what `AnimeResource` reads.
    pub fn page(self, page: u64, per_page: u64) -> (Pagination, Vec<Value>) {
        let pagination = paginate(self.items.len() as u64, page, per_page);
        let start = page.saturating_sub(1).saturating_mul(per_page);
        let items = self
            .items
            .into_iter()
            .skip(start as usize)
            .take(per_page as usize)
            .map(|item| materialize_accessors(&item))
            .collect();
        (pagination, items)
    }
}

pub(crate) fn storage_error(err: kuukan_store::StoreError) -> ApiErrorResponse {
    ApiErrorResponse(ApiError::Storage {
        error: Some(err.to_string()),
        report_url: None,
    })
}

fn passes_media_filters(item: &Value, sfw: bool, kids: Option<bool>, unapproved: bool) -> bool {
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
        .is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry.get("mal_id").and_then(Value::as_i64) == Some(i64::from(mal_id)))
        })
}

fn members_of(item: &Value) -> f64 {
    item.get("members")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NEG_INFINITY)
}

// ---------------------------------------------------------------------------
// Eloquent accessors (`App\Anime` getters)
// ---------------------------------------------------------------------------

/// Apply the `Anime` model accessors the PHP resource reads: `season`, `year`
/// (derived from `premiered`) and the adapted `broadcast` object. The store
/// keeps the JMS payload, so this runs before `AnimeResource`; the conversion
/// is idempotent.
pub fn materialize_accessors(item: &Value) -> Value {
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
    let open = broadcast.rfind(" (")?;
    let head = &broadcast[..open];
    let at = head.rfind(" at ")?;
    Some((head[..at].to_string(), head[at + 4..].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anime(mal_id: i64, members: Option<i64>) -> Value {
        let mut item = json!({"mal_id": mal_id, "approved": true});
        if let Some(members) = members {
            item["members"] = json!(members);
        }
        item
    }

    /// The pipeline's order is the module's job, not each caller's: filters
    /// run before ordering, and the pagination block counts what survived the
    /// filters rather than what the page holds.
    #[test]
    fn filters_then_orders_then_pages() {
        let collection = AnimeCollection::from_items(vec![
            anime(1, Some(10)),
            anime(2, Some(30)),
            anime(3, Some(20)),
            json!({"mal_id": 4, "approved": false, "members": 99}),
        ])
        .media_filters(true, None, false)
        .order_by_members(Members::Descending);

        assert_eq!(collection.len(), 3, "the unapproved entry is filtered out");

        let (pagination, items) = collection.page(1, 2);
        assert_eq!(pagination.last_visible_page, 2);
        assert!(pagination.has_next_page);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["mal_id"], 2);
        assert_eq!(items[1]["mal_id"], 3);
    }

    #[test]
    fn ascending_and_descending_are_mirror_images() {
        let items = vec![anime(1, Some(10)), anime(2, Some(30)), anime(3, Some(20))];
        let (_, desc) = AnimeCollection::from_items(items.clone())
            .order_by_members(Members::Descending)
            .page(1, 10);
        let (_, asc) = AnimeCollection::from_items(items)
            .order_by_members(Members::Ascending)
            .page(1, 10);
        let desc_ids: Vec<_> = desc.iter().map(|i| i["mal_id"].clone()).collect();
        let mut asc_ids: Vec<_> = asc.iter().map(|i| i["mal_id"].clone()).collect();
        asc_ids.reverse();
        assert_eq!(desc_ids, asc_ids);
    }

    /// Missing `members` sorts like BSON null: last descending, first
    /// ascending.
    #[test]
    fn missing_members_sorts_like_bson_null() {
        let items = vec![anime(1, Some(10)), anime(2, None), anime(3, Some(20))];
        let (_, desc) = AnimeCollection::from_items(items.clone())
            .order_by_members(Members::Descending)
            .page(1, 10);
        assert_eq!(desc[2]["mal_id"], 2);
        let (_, asc) = AnimeCollection::from_items(items)
            .order_by_members(Members::Ascending)
            .page(1, 10);
        assert_eq!(asc[0]["mal_id"], 2);
    }

    /// A page carries the accessors materialized, because that is what
    /// `AnimeResource` reads.
    #[test]
    fn page_items_carry_the_eloquent_accessors() {
        let item = json!({"mal_id": 1, "approved": true, "premiered": "Spring 2016"});
        let (_, items) = AnimeCollection::from_items(vec![item]).page(1, 10);
        assert_eq!(items[0]["season"], "spring");
        assert_eq!(items[0]["year"], 2016);
        assert!(items[0]["broadcast"].is_object());
    }

    #[test]
    fn pages_past_the_end_are_empty_but_still_paginate() {
        let collection = AnimeCollection::from_items(vec![anime(1, Some(1)), anime(2, Some(2))]);
        let (pagination, items) = collection.page(4, 2);
        assert!(items.is_empty());
        assert!(!pagination.has_next_page);
    }

    #[test]
    fn media_filters_match_the_scopes() {
        let normal = json!({
            "approved": true,
            "rating": "PG-13 - Teens 13 or older",
            "genres": [{"mal_id": 1}],
            "demographics": [{"mal_id": 27}],
        });
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

        let hentai = json!({
            "approved": true,
            "rating": "Rx - Hentai",
            "genres": [{"mal_id": 12}],
            "demographics": [],
        });
        assert!(!passes_media_filters(&hentai, true, None, false));
        assert!(passes_media_filters(&hentai, false, None, false));

        let kids = json!({
            "approved": true,
            "genres": [],
            "demographics": [{"mal_id": 15}],
        });
        assert!(passes_media_filters(&kids, false, Some(true), false));
        assert!(!passes_media_filters(&kids, false, Some(false), false));
        assert!(!passes_media_filters(&normal, false, Some(true), false));
        assert!(passes_media_filters(&normal, false, Some(false), false));
    }

    #[test]
    fn pagination_window_is_laravel_like() {
        let items: Vec<Value> = (0..5)
            .map(|id| json!({"mal_id": id, "approved": true}))
            .collect();
        let page = |page| AnimeCollection::from_items(items.clone()).page(page, 2).1;
        assert_eq!(page(1).len(), 2);
        assert_eq!(page(3).len(), 1);
        assert!(page(4).is_empty());
    }

    #[test]
    fn accessors_derive_season_year_and_broadcast() {
        let mapped = materialize_accessors(&json!({
            "mal_id": 2,
            "premiered": "Summer 2026",
            "broadcast": "Tuesdays at 23:00 (JST)",
            "aired": {"from": "2026-07-01T00:00:00+00:00", "to": null, "string": "Jul, 2026 to ?"},
        }));
        assert_eq!(mapped["season"], json!("summer"));
        assert_eq!(mapped["year"], json!(2026));
        assert_eq!(
            mapped["broadcast"],
            json!({"day": "Tuesdays", "time": "23:00", "timezone": "Asia/Tokyo", "string": "Tuesdays at 23:00 (JST)"})
        );

        // No premiered -> null season/year; null broadcast -> null object.
        let mapped = materialize_accessors(&json!({"mal_id": 3}));
        assert_eq!(mapped["season"], Value::Null);
        assert_eq!(mapped["year"], Value::Null);
        assert_eq!(
            mapped["broadcast"],
            json!({"day": null, "time": null, "timezone": null, "string": null})
        );

        // A broadcast without " at " keeps the string but nulls the parts.
        let mapped = materialize_accessors(&json!({"broadcast": "Not scheduled once per week"}));
        assert_eq!(
            mapped["broadcast"],
            json!({"day": null, "time": null, "timezone": null, "string": "Not scheduled once per week"})
        );

        // Idempotent for an already adapted document.
        let adapted = mapped.clone();
        assert_eq!(materialize_accessors(&adapted), adapted);
    }
}
