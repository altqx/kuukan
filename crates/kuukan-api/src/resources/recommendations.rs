//! Recommendation resources.
//!
//! - `/recommendations/anime` and `/recommendations/manga`
//!   (`QueryRecommendationsHandler`) use the default `ResultsResource`
//!   (`{"pagination": {last_visible_page, has_next_page}, "data": [...]}` with
//!   defaults `1`/`false`). Items are `Jikan\Model\Recommendations\RecommendationListItem`
//!   documents: `{mal_id, entry, content, date, user}`.
//! - `/anime/{id}/recommendations` and `/manga/{id}/recommendations`
//!   (`AnimeRecommendationsLookupHandler`, `MangaRecommendationsLookupHandler`)
//!   use `RecommendationsResource`, whose `toArray()` returns
//!   `$this['recommendations']` (a bare list). Items are
//!   `Jikan\Model\Common\Recommendation` documents: `{entry, url, votes}`.
//!   The stored/cached document is `{"recommendations": [...]}`; the shared
//!   raw-value port is [`crate::resources::misc::recommendations`].

use serde_json::{json, Value};

use crate::resources::misc;

fn get(payload: &Value, key: &str) -> Value {
    misc::get(payload, key)
}

/// `/recommendations/anime` and `/recommendations/manga` response body.
pub fn recommendations(payload: &Value) -> Value {
    misc::results(payload)
}

/// `RecommendationsResource::toArray()` for `/anime/{id}/recommendations` and
/// `/manga/{id}/recommendations`: the stored `recommendations` list.
pub fn entry_recommendations(payload: &Value) -> Vec<Value> {
    payload
        .get("recommendations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// `Jikan\Model\Recommendations\RecommendationListItem` JMS shape:
/// `{mal_id, entry, content, date, user}`.
pub fn recommendation_list_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "entry": get(payload, "entry"),
        "content": get(payload, "content"),
        "date": get(payload, "date"),
        "user": get(payload, "user"),
    })
}

/// `Jikan\Model\Common\Recommendation` JMS shape: `{entry, url, votes}`.
pub fn recommendation_item(payload: &Value) -> Value {
    json!({
        "entry": get(payload, "entry"),
        "url": get(payload, "url"),
        "votes": get(payload, "votes"),
    })
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

    fn list_item_doc() -> Value {
        json!({
            "mal_id": "4103-6675",
            "entry": [
                {
                    "mal_id": 4103,
                    "url": "https://myanimelist.net/anime/4103/Oval_x_Over",
                    "images": {"jpg": {"image_url": "x", "small_image_url": "t", "large_image_url": "l"}, "webp": {"image_url": "w", "small_image_url": "tw", "large_image_url": "lw"}},
                    "title": "Oval x Over"
                },
                {
                    "mal_id": 6675,
                    "url": "https://myanimelist.net/anime/6675/Redline",
                    "images": {"jpg": {"image_url": "x"}, "webp": {"image_url": "w"}},
                    "title": "Redline"
                }
            ],
            "content": "Oval x Over looks like a prototype version of Redline....",
            "date": "2022-06-20T17:21:22+00:00",
            "user": {"url": "https://myanimelist.net/profile/VBayer", "username": "VBayer"}
        })
    }

    #[test]
    fn recommendations_matches_results_resource_envelope() {
        let doc = json!({
            "results": [list_item_doc()],
            "last_visible_page": 2,
            "has_next_page": true
        });
        let out = recommendations(&doc);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 2, "has_next_page": true})
        );
        let item = &out["data"][0];
        assert_eq!(
            keys(item),
            vec!["content", "date", "entry", "mal_id", "user"]
        );
        assert_eq!(item["mal_id"], json!("4103-6675"));
        assert_eq!(item["entry"].as_array().unwrap().len(), 2);
        assert_eq!(item["user"]["username"], json!("VBayer"));
    }

    #[test]
    fn recommendations_defaults_when_empty() {
        let out = recommendations(&json!({}));
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 1, "has_next_page": false})
        );
        assert!(out["data"].is_null());
    }

    #[test]
    fn entry_recommendations_returns_bare_list() {
        let doc = json!({
            "recommendations": [
                {
                    "entry": {
                        "mal_id": 205,
                        "url": "https://myanimelist.net/anime/205/Samurai_Champloo",
                        "images": {"jpg": {"image_url": "x", "small_image_url": "t", "large_image_url": "l"}, "webp": {"image_url": "w", "small_image_url": "tw", "large_image_url": "lw"}},
                        "title": "Samurai Champloo"
                    },
                    "url": "https://myanimelist.net/recommendations/anime/1-205",
                    "votes": 118
                }
            ]
        });
        let items = entry_recommendations(&doc);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["entry"]["title"], json!("Samurai Champloo"));
        assert_eq!(items[0]["votes"], json!(118));
        assert!(entry_recommendations(&json!({})).is_empty());
    }

    #[test]
    fn recommendation_list_item_shape() {
        let out = recommendation_list_item(&list_item_doc());
        assert_eq!(
            keys(&out),
            vec!["content", "date", "entry", "mal_id", "user"]
        );
        assert_eq!(out["date"], json!("2022-06-20T17:21:22+00:00"));
        assert!(recommendation_list_item(&json!({}))["user"].is_null());
    }

    #[test]
    fn recommendation_item_shape() {
        let out = recommendation_item(&json!({
            "entry": {"mal_id": 205, "title": "Samurai Champloo"},
            "url": "https://myanimelist.net/recommendations/anime/1-205",
            "votes": 118
        }));
        assert_eq!(keys(&out), vec!["entry", "url", "votes"]);
        assert_eq!(out["votes"], json!(118));
        assert!(recommendation_item(&json!({}))["entry"].is_null());
    }
}
