//! Shared resources and helpers.
//!
//! Ported 1:1 from the PHP classes under `app/Http/Resources/V4/`:
//!
//! - [`results`] — `ResultsResource` (`{pagination:{last_visible_page,has_next_page}, data:results}`)
//! - [`news`] — `NewsResource` (shape identical to `ResultsResource`)
//! - [`forum`] — `ForumResource` (`$this['topics']`)
//! - [`pictures`] — `PicturesResource` (`$this['pictures']`)
//! - [`more_info`] — `MoreInfoResource` (`{moreinfo}`)
//! - [`recommendations`] — `RecommendationsResource` (`$this['recommendations']`)
//! - [`external_links`] — `ExternalLinksResource` (`$this->external_links`)
//! - [`streaming_links`] — `StreamingLinksResource` (`$this->streaming_links`)
//! - [`user_updates`] — `UserUpdatesResource` (`$this['users']`)
//! - [`reviews`] — `ReviewsResource` (`{pagination:{...}, data:results}`)
//!
//! The `get*` functions mirror Laravel's `JsonResource` ArrayAccess/`__get`
//! semantics: reading a missing (or `null`) key yields `null`; PHP's `??`
//! defaults are reproduced with [`get_or`].

use serde_json::{json, Value};

/// `$this[$key]` / `$this->{$key}`: clone the stored value, `null` when absent.
pub fn get(payload: &Value, key: &str) -> Value {
    payload.get(key).cloned().unwrap_or(Value::Null)
}

/// `$this[$key] ?? $default`: clone the stored value, `$default` when absent or null.
pub fn get_or(payload: &Value, key: &str, default: Value) -> Value {
    match payload.get(key) {
        None | Some(Value::Null) => default,
        Some(value) => value.clone(),
    }
}

/// First non-null value among `keys`, else `null`.
///
/// Used where the PHP resource reads an Eloquent *accessor* whose value is
/// stored under a different key (`favorites` -> `member_favorites`).
pub fn get_first(payload: &Value, keys: &[&str]) -> Value {
    for key in keys {
        if let Some(value) = payload.get(*key) {
            if !value.is_null() {
                return value.clone();
            }
        }
    }
    Value::Null
}

/// `{last_visible_page: $this['last_visible_page'] ?? 1, has_next_page: $this['has_next_page'] ?? false}`.
pub fn list_pagination(payload: &Value) -> Value {
    json!({
        "last_visible_page": get_or(payload, "last_visible_page", json!(1)),
        "has_next_page": get_or(payload, "has_next_page", json!(false)),
    })
}

/// `ResultsResource`: list envelope used by many endpoints.
pub fn results(payload: &Value) -> Value {
    json!({
        "pagination": list_pagination(payload),
        "data": get(payload, "results"),
    })
}

/// `NewsResource` (same shape as `ResultsResource`).
pub fn news(payload: &Value) -> Value {
    results(payload)
}

/// `ForumResource`: the stored document wraps topics under `topics`.
pub fn forum(payload: &Value) -> Value {
    get(payload, "topics")
}

/// `PicturesResource`.
pub fn pictures(payload: &Value) -> Value {
    get(payload, "pictures")
}

/// `MoreInfoResource`.
pub fn more_info(payload: &Value) -> Value {
    json!({
        "moreinfo": get(payload, "moreinfo"),
    })
}

/// `RecommendationsResource` (anime/manga shared).
pub fn recommendations(payload: &Value) -> Value {
    get(payload, "recommendations")
}

/// `ExternalLinksResource` (anime/manga/producer/user shared).
pub fn external_links(payload: &Value) -> Value {
    get(payload, "external_links")
}

/// `StreamingLinksResource`.
pub fn streaming_links(payload: &Value) -> Value {
    get(payload, "streaming_links")
}

/// `UserUpdatesResource`: the stored document wraps updates under `users`.
pub fn user_updates(payload: &Value) -> Value {
    get(payload, "users")
}

/// `ReviewsResource`.
///
/// Note: unlike `ResultsResource`, PHP has **no** `?? 1` / `?? false` defaults
/// here, so a missing pagination key is emitted as `null`.
pub fn reviews(payload: &Value) -> Value {
    json!({
        "pagination": {
            "last_visible_page": get(payload, "last_visible_page"),
            "has_next_page": get(payload, "has_next_page"),
        },
        "data": get(payload, "results"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_and_get_or_follow_php_null_coalescing() {
        let payload = json!({ "a": null, "b": 3 });

        assert_eq!(get(&payload, "a"), Value::Null);
        assert_eq!(get(&payload, "missing"), Value::Null);
        assert_eq!(get_or(&payload, "a", json!(1)), json!(1));
        assert_eq!(get_or(&payload, "missing", json!([])), json!([]));
        assert_eq!(get_or(&payload, "b", json!(1)), json!(3));

        // false is a value, not a missing one.
        let payload = json!({ "airing": false });
        assert_eq!(get_or(&payload, "airing", json!(true)), json!(false));

        let with_favorites = json!({ "favorites": null, "member_favorites": 5 });
        assert_eq!(
            get_first(&with_favorites, &["favorites", "member_favorites"]),
            json!(5)
        );
        assert_eq!(
            get_first(&payload, &["favorites", "member_favorites"]),
            Value::Null
        );
    }

    #[test]
    fn results_matches_dummy_results_document() {
        // tests/TestCase.php::dummyResultsDocument
        let payload = json!({
            "results": [{"mal_id": 1}],
            "has_next_page": true,
            "last_visible_page": 2,
            "request_hash": "request:anime:abc",
        });

        assert_eq!(
            results(&payload),
            json!({
                "pagination": {
                    "last_visible_page": 2,
                    "has_next_page": true,
                },
                "data": [{"mal_id": 1}],
            })
        );
    }

    #[test]
    fn results_defaults_pagination_like_php() {
        let payload = json!({});
        assert_eq!(
            results(&payload),
            json!({
                "pagination": {
                    "last_visible_page": 1,
                    "has_next_page": false,
                },
                "data": null,
            })
        );
    }

    #[test]
    fn news_is_identical_to_results() {
        let payload = json!({
            "results": [{"mal_id": 60609964, "title": "News"}],
            "last_visible_page": 1,
            "has_next_page": false,
        });
        assert_eq!(news(&payload), results(&payload));
    }

    #[test]
    fn forum_returns_topics_verbatim() {
        let payload = json!({
            "topics": [
                {
                    "mal_id": 2022869,
                    "url": "https://myanimelist.net/forum/?topicid=2022869",
                    "title": "What was the reception like when this first came out?",
                    "date": "2022-06-15T00:00:00+00:00",
                    "author_username": "NextUniverse",
                    "author_url": "https://myanimelist.net/profile/NextUniverse",
                    "comments": 7,
                    "last_comment": {
                        "url": "https://myanimelist.net/forum/?topicid=2022869&goto=lastpost",
                        "author_username": "Bacon_and_Eggs",
                        "author_url": "https://myanimelist.net/profile/Bacon_and_Eggs",
                        "date": "2022-06-19T06:26:00+00:00"
                    }
                }
            ]
        });
        assert_eq!(forum(&payload), payload["topics"]);
        // missing key is null, not [] (JsonResource array access semantics)
        assert_eq!(forum(&json!({})), Value::Null);
    }

    #[test]
    fn pictures_returns_pictures_verbatim() {
        // tests/Integration/AnimeControllerTest::testPictures
        let payload = json!({
            "pictures": [[
                {
                    "jpg": {
                        "image_url": "https://cdn.myanimelist.net/images/anime/7/3791.jpg",
                        "small_image_url": "https://cdn.myanimelist.net/images/anime/7/3791t.jpg",
                        "large_image_url": "https://cdn.myanimelist.net/images/anime/7/3791l.jpg"
                    },
                    "webp": {
                        "image_url": "https://cdn.myanimelist.net/images/anime/7/3791.webp",
                        "small_image_url": "https://cdn.myanimelist.net/images/anime/7/3791t.webp",
                        "large_image_url": "https://cdn.myanimelist.net/images/anime/7/3791l.webp"
                    }
                }
            ]]
        });
        assert_eq!(pictures(&payload), payload["pictures"]);
    }

    #[test]
    fn more_info_matches_php_shape() {
        let payload = json!({ "moreinfo": "asd" });
        assert_eq!(more_info(&payload), json!({ "moreinfo": "asd" }));
        assert_eq!(more_info(&json!({})), json!({ "moreinfo": null }));
    }

    #[test]
    fn recommendations_strip_the_envelope() {
        // tests/Integration/AnimeControllerTest::testRecommendations
        let payload = json!({
            "recommendations": [
                {
                    "entry": {
                        "mal_id": 205,
                        "url": "https://myanimelist.net/anime/205/Samurai_Champloo",
                        "images": {
                            "jpg": {
                                "image_url": "https://cdn.myanimelist.net/images/anime/1375/121599.jpg",
                                "small_image_url": "https://cdn.myanimelist.net/images/anime/1375/121599t.jpg",
                                "large_image_url": "https://cdn.myanimelist.net/images/anime/1375/121599l.jpg"
                            },
                            "webp": {
                                "image_url": "https://cdn.myanimelist.net/images/anime/1375/121599.webp",
                                "small_image_url": "https://cdn.myanimelist.net/images/anime/1375/121599t.webp",
                                "large_image_url": "https://cdn.myanimelist.net/images/anime/1375/121599l.webp"
                            }
                        },
                        "title": "Samurai Champloo"
                    },
                    "url": "https://myanimelist.net/recommendations/anime/1-205",
                    "votes": 118
                }
            ]
        });
        assert_eq!(recommendations(&payload), payload["recommendations"]);
    }

    #[test]
    fn external_and_streaming_links_return_raw_arrays() {
        let payload = json!({
            "external_links": [{"name": "Wikipedia", "url": "https://en.wikipedia.org/wiki/Cowboy_Bebop"}],
            "streaming_links": [{"name": "Crunchyroll", "url": "https://www.crunchyroll.com/cowboy-bebop"}],
        });
        assert_eq!(external_links(&payload), payload["external_links"]);
        assert_eq!(streaming_links(&payload), payload["streaming_links"]);
    }

    #[test]
    fn user_updates_return_users_array() {
        // tests/Integration/AnimeControllerTest::testAnimeUserUpdates
        let payload = json!({
            "users": [
                {
                    "user": {
                        "username": "Mar-E",
                        "url": "https://myanimelist.net/profile/Mar-E",
                        "images": {
                            "jpg": {"image_url": "https://cdn.myanimelist.net/images/userimages/12234611.jpg"},
                            "webp": {"image_url": "https://cdn.myanimelist.net/images/userimages/12234611.webp"}
                        }
                    },
                    "score": null,
                    "status": "Watching",
                    "episodes_seen": 16,
                    "episodes_total": 26,
                    "date": "2023-01-31T22:34:00+00:00"
                }
            ]
        });
        assert_eq!(user_updates(&payload), payload["users"]);
    }

    #[test]
    fn reviews_has_no_pagination_defaults() {
        // tests/Integration/AnimeControllerTest::testReviewsOne
        let payload = json!({
            "results": [{
                "mal_id": 7406,
                "url": "https://myanimelist.net/reviews.php?id=7406",
                "type": "anime",
                "reactions": {
                    "overall": 2112,
                    "nice": 2105,
                    "love_it": 3,
                    "funny": 1,
                    "confusing": 0,
                    "informative": 2,
                    "well_written": 1,
                    "creative": 0
                },
                "date": "2008-08-24T05:46:00+00:00",
                "review": "People who know me dd",
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
            }],
            "last_visible_page": 1,
            "has_next_page": false,
        });

        let mapped = reviews(&payload);
        assert_eq!(mapped["pagination"]["last_visible_page"], json!(1));
        assert_eq!(mapped["pagination"]["has_next_page"], json!(false));
        assert_eq!(mapped["data"], payload["results"]);
    }

    #[test]
    fn reviews_missing_pagination_is_null() {
        let mapped = reviews(&json!({ "results": [] }));
        assert_eq!(
            mapped,
            json!({
                "pagination": {"last_visible_page": null, "has_next_page": null},
                "data": []
            })
        );
    }
}
