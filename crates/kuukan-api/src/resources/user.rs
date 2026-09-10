//! User resources, ported from `app/Http/Resources/V4/`:
//!
//! - `ProfileResource.php`
//! - `ProfileFullResource.php`
//! - `ProfileAboutResource.php`
//! - `ProfileFavoritesResource.php`
//! - `ProfileHistoryResource.php`
//! - `ProfileLastUpdatesResource.php`
//! - `ProfileStatisticsResource.php`
//! - `UserProfileAnimeListResource.php` + `UserProfileAnimeListCollection.php`
//! - `UserProfileMangaListResource.php` + `UserProfileMangaListCollection.php`
//! - `UserCollection.php`
//!
//! Payloads are the stored/cached documents in JMS snake_case form.
//! Controllers/handlers that use them:
//! `User*LookupHandler`, `Query{Anime,Manga}ListOfUserHandler`.
//!
//! Collection mappers take the item list (`&[Value]`) like the sibling
//! resources; `*_response` builds the `{"data": [...]}` body the PHP
//! `ResourceCollection` produces.

use chrono::Datelike;
use kuukan_core::envelope;
use serde_json::{json, Value};

use crate::resources::misc::get;

const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn list_at(payload: &Value, key: &str) -> Vec<Value> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// `ProfileResource::toArray()`.
pub fn profile(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "username": get(payload, "username"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "last_online": get(payload, "last_online"),
        "gender": get(payload, "gender"),
        "birthday": get(payload, "birthday"),
        "location": get(payload, "location"),
        "joined": get(payload, "joined"),
    })
}

/// `ProfileFullResource::toArray()`.
pub fn profile_full(payload: &Value) -> Value {
    let mut out = profile(payload).as_object().cloned().unwrap_or_default();
    out.insert(
        "statistics".into(),
        json!({
            "anime": get(payload, "anime_stats"),
            "manga": get(payload, "manga_stats"),
        }),
    );
    out.insert("favorites".into(), get(payload, "favorites"));
    out.insert("updates".into(), get(payload, "last_updates"));
    out.insert("about".into(), get(payload, "about"));
    out.insert("external".into(), get(payload, "external_links"));
    Value::Object(out)
}

/// `ProfileAboutResource::toArray()`.
pub fn profile_about(payload: &Value) -> Value {
    json!({ "about": get(payload, "about") })
}

/// `ProfileFavoritesResource::toArray()` returns `$this->favorites` verbatim.
pub fn profile_favorites(payload: &Value) -> Value {
    get(payload, "favorites")
}

/// `ProfileHistoryResource::toArray()` returns `$this['history']`, a bare list.
pub fn profile_history(payload: &Value) -> Vec<Value> {
    list_at(payload, "history")
}

/// `ProfileLastUpdatesResource::toArray()` returns `$this->last_updates`.
pub fn profile_last_updates(payload: &Value) -> Value {
    get(payload, "last_updates")
}

/// `ProfileStatisticsResource::toArray()`.
pub fn profile_statistics(payload: &Value) -> Value {
    json!({
        "anime": get(payload, "anime_stats"),
        "manga": get(payload, "manga_stats"),
    })
}

// ---------------------------------------------------------------------------
// User list item resources
// ---------------------------------------------------------------------------

/// PHP `(string) $value` for the `tags` column (`null` becomes `""`).
fn tags_string(value: Option<&Value>) -> Value {
    match value {
        Some(Value::String(s)) => Value::String(s.clone()),
        Some(Value::Null) | None => Value::String(String::new()),
        Some(Value::Bool(true)) => Value::String("1".into()),
        Some(Value::Bool(false)) => Value::String(String::new()),
        Some(Value::Number(n)) => Value::String(n.to_string()),
        Some(Value::Array(_)) | Some(Value::Object(_)) => Value::String("Array".into()),
    }
}

/// PHP `strtolower($this['season_name'])` (`null` becomes `""`).
fn season_lower(payload: &Value) -> Value {
    match payload.get("season_name") {
        Some(Value::String(s)) => Value::String(s.to_lowercase()),
        Some(Value::Null) | None => Value::String(String::new()),
        Some(other) => Value::String(other.to_string().to_lowercase()),
    }
}

/// `Jikan\Helper\Constants::USER_ANIME_LIST_*` airing status labels.
fn anime_airing_status(status: Option<i64>) -> Value {
    match status {
        Some(1) => Value::String("Currently Airing".into()),
        Some(2) => Value::String("Finished Airing".into()),
        Some(3) => Value::String("Not yet aired".into()),
        _ => Value::Null,
    }
}

/// `Jikan\Helper\Constants::USER_MANGA_LIST_*` publishing status labels.
fn manga_publishing_status(status: Option<i64>) -> Value {
    match status {
        Some(1) => Value::String("Publishing".into()),
        Some(2) => Value::String("Finished".into()),
        Some(3) => Value::String("Not yet published".into()),
        Some(4) => Value::String("On Hiatus".into()),
        Some(5) => Value::String("Discontinued".into()),
        _ => Value::Null,
    }
}

/// `strtotime()` + `date('M j, Y')` in the app timezone (UTC), then parsed by
/// `Jikan\Model\Common\DateRange` (`Parser::parseDate`). Returns
/// `(day, month, year)` of the calendar date, defaulting to the PHP
/// `date('M j, Y', false)` behavior (`Jan 1, 1970`) for unparseable input.
fn php_calendar_date(raw: &Value) -> (i64, i64, i64) {
    let parsed = raw.as_str().and_then(|s| {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            let d = dt.with_timezone(&chrono::Utc).date_naive();
            return Some((d.day() as i64, d.month() as i64, d.year() as i64));
        }
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(|d| (d.day() as i64, d.month() as i64, d.year() as i64))
    });
    parsed.unwrap_or((1, 1, 1970))
}

fn display_date(parts: (i64, i64, i64)) -> String {
    format!(
        "{} {}, {}",
        MONTH_ABBR[(parts.1 - 1) as usize],
        parts.0,
        parts.2
    )
}

fn atom(parts: (i64, i64, i64)) -> String {
    format!(
        "{:04}-{:02}-{:02}T00:00:00+00:00",
        parts.2, parts.1, parts.0
    )
}

fn prop(parts: Option<(i64, i64, i64)>) -> Value {
    match parts {
        Some((day, month, year)) => json!({ "day": day, "month": month, "year": year }),
        None => json!({ "day": null, "month": null, "year": null }),
    }
}

/// The `{from, to, prop, string}` block built by both user list resources.
fn date_range(start: &Value, end: &Value) -> Value {
    let start_not_available = match start {
        Value::Null => true,
        Value::String(s) => s == "Not available",
        _ => false,
    };
    let end_unknown = match end {
        Value::Null => true,
        Value::String(s) => s == "?",
        _ => false,
    };

    let start_parts = if start_not_available {
        None
    } else {
        Some(php_calendar_date(start))
    };
    let end_parts = if end_unknown {
        None
    } else {
        Some(php_calendar_date(end))
    };

    let start_str = match start_parts {
        Some(parts) => display_date(parts),
        None => "Not available".to_string(),
    };
    let end_str = match end_parts {
        Some(parts) => display_date(parts),
        None => "?".to_string(),
    };

    json!({
        "from": start_parts.map(atom),
        "to": end_parts.map(atom),
        "prop": {
            "from": prop(start_parts),
            "to": prop(end_parts),
        },
        "string": format!("{start_str} to {end_str}"),
    })
}

fn is_status(value: &Value, key: &str, status: i64) -> bool {
    value.get(key).and_then(Value::as_i64) == Some(status)
}

/// `UserProfileAnimeListResource::toArray()`.
pub fn user_profile_anime_list(payload: &Value) -> Value {
    let airing_status = payload.get("airing_status").and_then(Value::as_i64);
    json!({
        "watching_status": get(payload, "watching_status"),
        "score": get(payload, "score"),
        "episodes_watched": get(payload, "watched_episodes"),
        "tags": tags_string(payload.get("tags")),
        "is_rewatching": get(payload, "is_rewatching"),
        "watch_start_date": get(payload, "watch_start_date"),
        "watch_end_date": get(payload, "watch_end_date"),
        "days": get(payload, "days"),
        "storage": get(payload, "storage"),
        "priority": get(payload, "priority"),
        "anime": {
            "mal_id": get(payload, "mal_id"),
            "title": get(payload, "title"),
            "url": get(payload, "url"),
            "images": get(payload, "images"),
            "type": get(payload, "type"),
            "season": season_lower(payload),
            "year": get(payload, "season_year"),
            "episodes": get(payload, "total_episodes"),
            "rating": get(payload, "rating"),
            "status": anime_airing_status(airing_status),
            "airing": is_status(payload, "airing_status", 1),
            "aired": date_range(
                &get(payload, "start_date"),
                &get(payload, "end_date"),
            ),
            "studios": get(payload, "studios"),
            "licensors": get(payload, "licensors"),
            "genres": get(payload, "genres"),
            "demographics": get(payload, "demographics"),
        },
    })
}

/// `UserProfileAnimeListCollection` item mapping (`$collects` is
/// `UserProfileAnimeListResource`).
pub fn user_profile_anime_list_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(user_profile_anime_list).collect()
}

/// `UserProfileAnimeListCollection::toArray()` (`{"data": [...]}`).
pub fn user_profile_anime_list_response(items: &[Value]) -> Value {
    envelope::data(user_profile_anime_list_collection(items))
}

/// `UserProfileMangaListResource::toArray()`.
pub fn user_profile_manga_list(payload: &Value) -> Value {
    let publishing_status = payload.get("publishing_status").and_then(Value::as_i64);
    json!({
        "reading_status": get(payload, "reading_status"),
        "score": get(payload, "score"),
        "chapters_read": get(payload, "read_chapters"),
        "volumes_read": get(payload, "read_volumes"),
        "tags": tags_string(payload.get("tags")),
        "is_rereading": get(payload, "is_rereading"),
        "read_start_date": get(payload, "read_start_date"),
        "read_end_date": get(payload, "read_end_date"),
        "days": get(payload, "days"),
        "retail": get(payload, "retail"),
        "priority": get(payload, "priority"),
        "manga": {
            "mal_id": get(payload, "mal_id"),
            "title": get(payload, "title"),
            "url": get(payload, "url"),
            "images": get(payload, "images"),
            "type": get(payload, "type"),
            "chapters": get(payload, "total_chapters"),
            "volumes": get(payload, "total_volumes"),
            "status": manga_publishing_status(publishing_status),
            "publishing": is_status(payload, "publishing_status", 1),
            "published": date_range(
                &get(payload, "start_date"),
                &get(payload, "end_date"),
            ),
            "magazines": get(payload, "magazines"),
            "genres": get(payload, "genres"),
            "demographics": get(payload, "demographics"),
        },
    })
}

/// `UserProfileMangaListCollection` item mapping.
pub fn user_profile_manga_list_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(user_profile_manga_list).collect()
}

/// `UserProfileMangaListCollection::toArray()` (`{"data": [...]}`).
pub fn user_profile_manga_list_response(items: &[Value]) -> Value {
    envelope::data(user_profile_manga_list_collection(items))
}

/// `UserCollection` item mapping (`$collects` is `ProfileResource`).
pub fn user_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(profile).collect()
}

/// `UserCollection::toArray()` (`{"data": [...]}`, no pagination block).
pub fn user_search(items: &[Value]) -> Value {
    envelope::data(user_collection(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("object")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    fn profile_doc() -> Value {
        json!({
            "mal_id": 1,
            "username": "nekomata1037",
            "url": "https://myanimelist.net/profile/nekomata1037",
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/userimages/1.jpg"},
                "webp": {"image_url": "https://cdn.myanimelist.net/images/userimages/1.webp"}
            },
            "last_online": "2024-01-06T00:00:00+00:00",
            "gender": "Female",
            "birthday": "1990-10-01T00:00:00+00:00",
            "location": "Mars",
            "joined": "2000-10-01T00:00:00+00:00",
            "anime_stats": {
                "days_watched": 106.8,
                "mean_score": 8.21,
                "watching": 44,
                "completed": 449,
                "on_hold": 15,
                "dropped": 3,
                "plan_to_watch": 426,
                "total_entries": 937,
                "rewatched": 22,
                "episodes_watched": 6305
            },
            "manga_stats": {
                "days_read": 91,
                "mean_score": 8.45,
                "reading": 380,
                "completed": 145,
                "on_hold": 3,
                "dropped": 1,
                "plan_to_read": 58,
                "total_entries": 587,
                "reread": 1,
                "chapters_read": 16263,
                "volumes_read": 777
            },
            "favorites": {
                "anime": [{"mal_id": 9253, "url": "https://myanimelist.net/anime/9253/Steins_Gate", "images": {"jpg": {"image_url": "a"}, "webp": {"image_url": "b"}}, "title": "Steins;Gate", "type": "TV", "start_year": 2011}],
                "manga": [],
                "characters": [],
                "people": []
            },
            "last_updates": {"anime": [], "manga": []},
            "external_links": [{"name": "Twitter", "url": "https://twitter.com/x"}],
            "about": "Hello"
        })
    }

    #[test]
    fn profile_matches_php_key_set() {
        let out = profile(&profile_doc());
        assert_eq!(
            keys(&out),
            vec![
                "birthday",
                "gender",
                "images",
                "joined",
                "last_online",
                "location",
                "mal_id",
                "url",
                "username",
            ]
        );
        assert_eq!(out["mal_id"], json!(1));
        assert_eq!(out["username"], json!("nekomata1037"));
        assert_eq!(out["last_online"], json!("2024-01-06T00:00:00+00:00"));
    }

    #[test]
    fn profile_missing_keys_emit_nulls() {
        let out = profile(&json!({}));
        assert_eq!(
            keys(&out),
            vec![
                "birthday",
                "gender",
                "images",
                "joined",
                "last_online",
                "location",
                "mal_id",
                "url",
                "username",
            ]
        );
        assert!(out["mal_id"].is_null());
        assert!(out["username"].is_null());
        assert!(out["images"].is_null());
    }

    #[test]
    fn profile_full_matches_php_key_set() {
        let out = profile_full(&profile_doc());
        assert_eq!(
            keys(&out),
            vec![
                "about",
                "birthday",
                "external",
                "favorites",
                "gender",
                "images",
                "joined",
                "last_online",
                "location",
                "mal_id",
                "statistics",
                "updates",
                "url",
                "username",
            ]
        );
        assert_eq!(out["statistics"]["anime"]["days_watched"], json!(106.8));
        assert_eq!(out["statistics"]["manga"]["days_read"], json!(91));
        assert_eq!(out["updates"]["anime"], json!([]));
        assert_eq!(out["external"][0]["name"], json!("Twitter"));
        assert_eq!(out["about"], json!("Hello"));
        // `anime_stats`/`manga_stats` are only exposed under `statistics`.
        assert!(out.get("anime_stats").is_none());
    }

    #[test]
    fn profile_full_missing_sections_are_null() {
        let out = profile_full(&json!({"mal_id": 1}));
        assert!(out["statistics"].get("anime").is_some());
        assert!(out["favorites"].is_null());
        assert!(out["updates"].is_null());
        assert!(out["about"].is_null());
        assert!(out["external"].is_null());
    }

    #[test]
    fn profile_about_and_statistics() {
        assert_eq!(profile_about(&profile_doc()), json!({"about": "Hello"}));
        assert_eq!(profile_about(&json!({})), json!({"about": null}));
        let stats = profile_statistics(&profile_doc());
        assert_eq!(keys(&stats), vec!["anime", "manga"]);
        assert_eq!(stats["anime"]["episodes_watched"], json!(6305));
    }

    #[test]
    fn profile_favorites_and_updates_pass_through() {
        let doc = profile_doc();
        assert_eq!(profile_favorites(&doc), doc["favorites"]);
        assert_eq!(profile_last_updates(&doc), doc["last_updates"]);
        assert!(profile_favorites(&json!({})).is_null());
        assert!(profile_last_updates(&json!({})).is_null());
    }

    #[test]
    fn profile_history_returns_bare_list() {
        let doc = json!({
            "history": [
                {
                    "entry": {"mal_id": 2, "type": "anime", "name": "Cowboy Bebop", "url": "https://myanimelist.net/anime/1"},
                    "increment": 1,
                    "date": "2024-01-01T00:00:00+00:00"
                }
            ]
        });
        let items = profile_history(&doc);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["entry"]["name"], json!("Cowboy Bebop"));
        assert!(profile_history(&json!({})).is_empty());
    }

    fn anime_list_item() -> Value {
        json!({
            "mal_id": 9253,
            "title": "Steins;Gate",
            "url": "https://myanimelist.net/anime/9253/Steins_Gate",
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/5/73199.jpg", "small_image_url": "https://cdn.myanimelist.net/images/anime/5/73199t.jpg", "large_image_url": "https://cdn.myanimelist.net/images/anime/5/73199l.jpg"},
                "webp": {"image_url": "https://cdn.myanimelist.net/images/anime/5/73199.webp", "small_image_url": "https://cdn.myanimelist.net/images/anime/5/73199t.webp", "large_image_url": "https://cdn.myanimelist.net/images/anime/5/73199l.webp"}
            },
            "video_url": "https://myanimelist.net/anime/9253/Steins_Gate/video",
            "watching_status": 2,
            "score": 10,
            "tags": "favorite",
            "is_rewatching": false,
            "watched_episodes": 24,
            "total_episodes": 24,
            "airing_status": 2,
            "has_episode_video": true,
            "has_promo_video": true,
            "has_video": true,
            "type": "TV",
            "rating": "R - 17+ (violence & profanity)",
            "start_date": "2011-04-06T00:00:00+00:00",
            "end_date": "2011-09-14T00:00:00+00:00",
            "watch_start_date": "2015-01-01T00:00:00+00:00",
            "watch_end_date": "2015-02-01T00:00:00+00:00",
            "days": "24",
            "storage": null,
            "priority": "Low",
            "added_to_list": true,
            "season_name": "Spring",
            "season_year": 2011,
            "studios": [{"mal_id": 314, "type": "anime", "name": "White Fox", "url": "https://myanimelist.net/anime/producer/314/White_Fox"}],
            "licensors": [],
            "genres": [{"mal_id": 24, "type": "anime", "name": "Sci-Fi", "url": "https://myanimelist.net/anime/genre/24/Sci-Fi"}],
            "demographics": []
        })
    }

    #[test]
    fn user_profile_anime_list_matches_php_key_set() {
        let out = user_profile_anime_list(&anime_list_item());
        assert_eq!(
            keys(&out),
            vec![
                "anime",
                "days",
                "episodes_watched",
                "is_rewatching",
                "priority",
                "score",
                "storage",
                "tags",
                "watch_end_date",
                "watch_start_date",
                "watching_status",
            ]
        );
        assert_eq!(out["watching_status"], json!(2));
        assert_eq!(out["episodes_watched"], json!(24));
        assert_eq!(out["tags"], json!("favorite"));
        assert_eq!(
            keys(&out["anime"]),
            vec![
                "aired",
                "airing",
                "demographics",
                "episodes",
                "genres",
                "images",
                "licensors",
                "mal_id",
                "rating",
                "season",
                "status",
                "studios",
                "title",
                "type",
                "url",
                "year",
            ]
        );
        assert_eq!(out["anime"]["season"], json!("spring"));
        assert_eq!(out["anime"]["year"], json!(2011));
        assert_eq!(out["anime"]["episodes"], json!(24));
        assert_eq!(out["anime"]["status"], json!("Finished Airing"));
        assert_eq!(out["anime"]["airing"], json!(false));
        assert_eq!(out["anime"]["aired"]["from"], json!("2011-04-06T00:00:00+00:00"));
        assert_eq!(out["anime"]["aired"]["to"], json!("2011-09-14T00:00:00+00:00"));
        assert_eq!(out["anime"]["aired"]["prop"]["from"], json!({"day": 6, "month": 4, "year": 2011}));
        assert_eq!(out["anime"]["aired"]["prop"]["to"], json!({"day": 14, "month": 9, "year": 2011}));
        assert_eq!(out["anime"]["aired"]["string"], json!("Apr 6, 2011 to Sep 14, 2011"));
    }

    #[test]
    fn user_profile_anime_list_default_dates() {
        let item = json!({
            "mal_id": 1,
            "airing_status": 1,
            "season_name": null,
            "tags": null,
            "start_date": null,
            "end_date": null
        });
        let out = user_profile_anime_list(&item);
        assert_eq!(out["tags"], json!(""));
        assert_eq!(out["anime"]["season"], json!(""));
        assert_eq!(out["anime"]["status"], json!("Currently Airing"));
        assert_eq!(out["anime"]["airing"], json!(true));
        assert!(out["anime"]["aired"]["from"].is_null());
        assert!(out["anime"]["aired"]["to"].is_null());
        assert_eq!(
            out["anime"]["aired"]["prop"]["from"],
            json!({"day": null, "month": null, "year": null})
        );
        assert_eq!(out["anime"]["aired"]["string"], json!("Not available to ?"));
    }

    #[test]
    fn user_profile_anime_list_unknown_status_maps_to_null() {
        let out = user_profile_anime_list(&json!({"airing_status": 9}));
        assert!(out["anime"]["status"].is_null());
        assert_eq!(out["anime"]["airing"], json!(false));
    }

    #[test]
    fn user_profile_anime_list_collection_maps_items_and_wraps_data() {
        let items = vec![anime_list_item(), anime_list_item()];
        let mapped = user_profile_anime_list_collection(&items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0]["anime"]["mal_id"], json!(9253));
        assert_eq!(
            user_profile_anime_list_response(&items),
            json!({"data": mapped})
        );
        assert_eq!(
            user_profile_anime_list_response(&[]),
            json!({"data": []})
        );
    }

    fn manga_list_item() -> Value {
        json!({
            "mal_id": 642,
            "title": "Vinland Saga",
            "url": "https://myanimelist.net/manga/642/Vinland_Saga",
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/manga/2/188925.jpg", "small_image_url": "t", "large_image_url": "l"},
                "webp": {"image_url": "w", "small_image_url": "tw", "large_image_url": "lw"}
            },
            "reading_status": 1,
            "score": 9,
            "read_chapters": 75,
            "read_volumes": 8,
            "total_chapters": 200,
            "total_volumes": 27,
            "publishing_status": 1,
            "is_rereading": false,
            "tags": null,
            "start_date": "2005-04-13T00:00:00+00:00",
            "end_date": null,
            "read_start_date": "2020-01-01T00:00:00+00:00",
            "read_end_date": null,
            "days": "30",
            "retail": null,
            "priority": "High",
            "added_to_list": true,
            "magazines": [{"mal_id": 2, "type": "manga", "name": "Monthly Afternoon", "url": "https://myanimelist.net/manga/magazine/2/Monthly_Afternoon"}],
            "genres": [],
            "demographics": []
        })
    }

    #[test]
    fn user_profile_manga_list_matches_php_key_set() {
        let out = user_profile_manga_list(&manga_list_item());
        assert_eq!(
            keys(&out),
            vec![
                "chapters_read",
                "days",
                "is_rereading",
                "manga",
                "priority",
                "read_end_date",
                "read_start_date",
                "reading_status",
                "retail",
                "score",
                "tags",
                "volumes_read",
            ]
        );
        assert_eq!(out["chapters_read"], json!(75));
        assert_eq!(out["volumes_read"], json!(8));
        assert_eq!(out["tags"], json!(""));
        assert_eq!(
            keys(&out["manga"]),
            vec![
                "chapters",
                "demographics",
                "genres",
                "images",
                "magazines",
                "mal_id",
                "published",
                "publishing",
                "status",
                "title",
                "type",
                "url",
                "volumes",
            ]
        );
        assert_eq!(out["manga"]["chapters"], json!(200));
        assert_eq!(out["manga"]["volumes"], json!(27));
        assert_eq!(out["manga"]["status"], json!("Publishing"));
        assert_eq!(out["manga"]["publishing"], json!(true));
        assert_eq!(
            out["manga"]["published"]["string"],
            json!("Apr 13, 2005 to ?")
        );
        assert_eq!(out["manga"]["published"]["from"], json!("2005-04-13T00:00:00+00:00"));
        assert!(out["manga"]["published"]["to"].is_null());
        assert_eq!(out["manga"]["published"]["prop"]["to"], json!({"day": null, "month": null, "year": null}));
    }

    #[test]
    fn user_profile_manga_list_collection_wraps_data() {
        let items = vec![manga_list_item()];
        let mapped = user_profile_manga_list_collection(&items);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0]["manga"]["status"], json!("Publishing"));
        assert_eq!(user_profile_manga_list_response(&items), json!({"data": mapped}));
        assert_eq!(user_profile_manga_list_response(&[]), json!({"data": []}));
    }

    #[test]
    fn user_collection_maps_profiles() {
        let items = vec![
            json!({"mal_id": 1, "username": "a", "url": "u", "images": {"jpg": {"image_url": "x"}}}),
            json!({"username": "b"}),
        ];
        let mapped = user_collection(&items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0]["username"], json!("a"));
        assert_eq!(mapped[1]["mal_id"], json!(null));
        assert_eq!(keys(&user_search(&items)), vec!["data"]);
        assert_eq!(user_search(&items)["data"].as_array().unwrap().len(), 2);
    }
}
