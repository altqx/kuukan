//! Review resources.
//!
//! - `ReviewsResource.php` is used by `AnimeReviewsLookupHandler` /
//!   `MangaReviewsLookupHandler` (`/anime/{id}/reviews`,
//!   `/manga/{id}/reviews`). Unlike `ResultsResource` it does **not** default
//!   the pagination fields, so a missing key stays `null`. The shared port
//!   lives in [`crate::resources::misc::reviews`].
//! - `/reviews/anime` and `/reviews/manga` (`QueryReviewsHandler`) use the
//!   default `ResultsResource` (`last_visible_page ?? 1`,
//!   `has_next_page ?? false`). Items are `FullAnimeReview`/`FullMangaReview`
//!   documents (they additionally carry `entry`).
//!
//! The per-item helpers below document/normalize the review model JMS shapes.

use serde_json::{json, Value};

use crate::resources::misc;

fn get(payload: &Value, key: &str) -> Value {
    misc::get(payload, key)
}

/// `ReviewsResource::toArray()` (`/anime/{id}/reviews`, `/manga/{id}/reviews`).
pub fn reviews_resource(payload: &Value) -> Value {
    misc::reviews(payload)
}

/// Default `ResultsResource` body for `/reviews/anime` and `/reviews/manga`.
pub fn reviews(payload: &Value) -> Value {
    misc::results(payload)
}

/// `Jikan\Model\Anime\AnimeReview` JMS shape (per-entry review).
pub fn anime_review_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "type": get(payload, "type"),
        "reactions": get(payload, "reactions"),
        "date": get(payload, "date"),
        "review": get(payload, "review"),
        "score": get(payload, "score"),
        "tags": get(payload, "tags"),
        "is_spoiler": get(payload, "is_spoiler"),
        "is_preliminary": get(payload, "is_preliminary"),
        "episodes_watched": get(payload, "episodes_watched"),
        "user": get(payload, "user"),
    })
}

/// `Jikan\Model\Manga\MangaReview` JMS shape (per-entry review).
pub fn manga_review_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "type": get(payload, "type"),
        "reactions": get(payload, "reactions"),
        "date": get(payload, "date"),
        "review": get(payload, "review"),
        "score": get(payload, "score"),
        "tags": get(payload, "tags"),
        "is_spoiler": get(payload, "is_spoiler"),
        "is_preliminary": get(payload, "is_preliminary"),
        "chapters_read": get(payload, "chapters_read"),
        "user": get(payload, "user"),
    })
}

/// `Jikan\Model\Reviews\FullAnimeReview` JMS shape (`/reviews/anime` items):
/// the per-entry review plus `entry`.
pub fn full_anime_review_item(payload: &Value) -> Value {
    let mut out = anime_review_item(payload)
        .as_object()
        .cloned()
        .unwrap_or_default();
    out.insert("entry".into(), get(payload, "entry"));
    Value::Object(out)
}

/// `Jikan\Model\Reviews\FullMangaReview` JMS shape (`/reviews/manga` items):
/// the per-entry review plus `entry`.
pub fn full_manga_review_item(payload: &Value) -> Value {
    let mut out = manga_review_item(payload)
        .as_object()
        .cloned()
        .unwrap_or_default();
    out.insert("entry".into(), get(payload, "entry"));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }

    fn anime_review_doc() -> Value {
        json!({
            "mal_id": 7406,
            "url": "https://myanimelist.net/reviews.php?id=7406",
            "type": "anime",
            "reactions": {
                "overall": 2112, "nice": 2105, "love_it": 3, "funny": 1,
                "confusing": 0, "informative": 2, "well_written": 1, "creative": 0
            },
            "date": "2008-08-24T05:46:00+00:00",
            "review": "People who know me",
            "score": 10,
            "tags": ["Recommended"],
            "is_spoiler": false,
            "is_preliminary": false,
            "episodes_watched": null,
            "user": {
                "url": "https://myanimelist.net/profile/TheLlama",
                "username": "TheLlama",
                "images": {
                    "jpg": {"image_url": "https://cdn.myanimelist.net/images/userimages/11081.jpg"},
                    "webp": {"image_url": "https://cdn.myanimelist.net/images/userimages/11081.webp"}
                }
            }
        })
    }

    #[test]
    fn reviews_resource_keeps_pagination_verbatim() {
        let doc = json!({
            "results": [anime_review_doc()],
            "last_visible_page": 2,
            "has_next_page": true
        });
        let out = reviews_resource(&doc);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 2, "has_next_page": true})
        );
        let item = &out["data"][0];
        assert_eq!(
            keys(item),
            vec![
                "date",
                "episodes_watched",
                "is_preliminary",
                "is_spoiler",
                "mal_id",
                "reactions",
                "review",
                "score",
                "tags",
                "type",
                "url",
                "user"
            ]
        );
        assert!(item["episodes_watched"].is_null());
        assert_eq!(item["user"]["username"], json!("TheLlama"));
    }

    #[test]
    fn reviews_resource_missing_pagination_is_null() {
        let out = reviews_resource(&json!({"results": []}));
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": null, "has_next_page": null})
        );
        assert_eq!(out["data"], json!([]));
    }

    fn full_anime_review_doc() -> Value {
        let mut doc = anime_review_doc();
        doc["episodes_watched"] = json!(12);
        doc["entry"] = json!({
            "mal_id": 43470,
            "url": "https://myanimelist.net/anime/43470",
            "images": {"jpg": {"image_url": "x", "small_image_url": "t", "large_image_url": "l"}, "webp": {"image_url": "w", "small_image_url": "tw", "large_image_url": "lw"}},
            "title": "Rikei"
        });
        doc
    }

    #[test]
    fn reviews_uses_results_resource_defaults() {
        let out = reviews(&json!({
            "results": [full_anime_review_doc()],
            "last_visible_page": 3,
            "has_next_page": true
        }));
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 3, "has_next_page": true})
        );
        assert_eq!(out["data"][0]["episodes_watched"], json!(12));
        assert_eq!(out["data"][0]["entry"]["mal_id"], json!(43470));
        // Missing metadata falls back to the ResultsResource defaults.
        let empty = reviews(&json!({}));
        assert_eq!(
            empty,
            json!({
                "pagination": {"last_visible_page": 1, "has_next_page": false},
                "data": null
            })
        );
    }

    #[test]
    fn anime_review_item_shape() {
        let out = anime_review_item(&anime_review_doc());
        assert_eq!(
            keys(&out),
            vec![
                "date",
                "episodes_watched",
                "is_preliminary",
                "is_spoiler",
                "mal_id",
                "reactions",
                "review",
                "score",
                "tags",
                "type",
                "url",
                "user"
            ]
        );
        assert_eq!(out["score"], json!(10));
    }

    #[test]
    fn manga_review_item_shape() {
        let out = manga_review_item(&json!({
            "mal_id": 448579,
            "url": "https://myanimelist.net/reviews.php?id=448579",
            "type": "manga",
            "reactions": {"overall": 0, "nice": 0, "love_it": 0, "funny": 0, "confusing": 0, "informative": 0, "well_written": 0, "creative": 0},
            "date": "2022-06-20T12:13:00+00:00",
            "review": "Its good",
            "score": 5,
            "tags": ["Recommended"],
            "is_spoiler": false,
            "is_preliminary": false,
            "chapters_read": 75,
            "user": {"url": "u", "username": "helmy47", "images": {"jpg": {"image_url": "i"}}}
        }));
        assert_eq!(
            keys(&out),
            vec![
                "chapters_read",
                "date",
                "is_preliminary",
                "is_spoiler",
                "mal_id",
                "reactions",
                "review",
                "score",
                "tags",
                "type",
                "url",
                "user"
            ]
        );
        assert_eq!(out["chapters_read"], json!(75));
        assert!(out.get("episodes_watched").is_none());
    }

    #[test]
    fn full_review_items_add_entry() {
        let anime = full_anime_review_item(&full_anime_review_doc());
        assert_eq!(
            keys(&anime),
            vec![
                "date",
                "entry",
                "episodes_watched",
                "is_preliminary",
                "is_spoiler",
                "mal_id",
                "reactions",
                "review",
                "score",
                "tags",
                "type",
                "url",
                "user"
            ]
        );
        assert_eq!(anime["entry"]["mal_id"], json!(43470));
        let manga = full_manga_review_item(
            &json!({"mal_id": 1, "chapters_read": 75, "entry": {"mal_id": 2}}),
        );
        assert_eq!(
            keys(&manga),
            vec![
                "chapters_read",
                "date",
                "entry",
                "is_preliminary",
                "is_spoiler",
                "mal_id",
                "reactions",
                "review",
                "score",
                "tags",
                "type",
                "url",
                "user"
            ]
        );
    }
}
