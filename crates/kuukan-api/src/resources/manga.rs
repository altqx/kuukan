//! Manga response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/`:
//!
//! - [`manga`] — `MangaResource`
//! - [`manga_full`] — `MangaFullResource`
//! - [`manga_characters`] — `MangaCharactersResource`
//! - [`manga_statistics`] — `MangaStatisticsResource`
//! - [`manga_relations`] — `MangaRelationsResource`
//!
//! Shared resources used by manga endpoints are re-exported as `manga_*`
//! wrappers around the canonical mappers in [`crate::resources::misc`]:
//! `NewsResource`, `ForumResource`, `PicturesResource`, `MoreInfoResource`,
//! `RecommendationsResource`, `ExternalLinksResource`, `UserUpdatesResource`
//! and `ReviewsResource`.
//!
//! Quirk preserved: `scored` duplicates `score` (deprecated in PHP, kept until
//! 4.1+). `related` is normalized with
//! [`kuukan_core::util::normalize_related`] for the full payload.

use kuukan_core::util::normalize_related;
use serde_json::{json, Value};

use crate::resources::misc::{self, get, get_or};

/// `MangaResource`.
pub fn manga(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "approved": get_or(payload, "approved", json!(true)),
        "titles": get_or(payload, "titles", json!([])),
        "title": get(payload, "title"),
        "title_english": get(payload, "title_english"),
        "title_japanese": get(payload, "title_japanese"),
        "title_synonyms": get(payload, "title_synonyms"),
        "type": get(payload, "type"),
        "chapters": get(payload, "chapters"),
        "volumes": get(payload, "volumes"),
        "status": get(payload, "status"),
        "publishing": get(payload, "publishing"),
        "published": get(payload, "published"),
        "score": get(payload, "score"),
        "scored": get(payload, "score"),
        "scored_by": get(payload, "scored_by"),
        "rank": get(payload, "rank"),
        "popularity": get(payload, "popularity"),
        "members": get(payload, "members"),
        "favorites": get(payload, "favorites"),
        "synopsis": get(payload, "synopsis"),
        "background": get(payload, "background"),
        "authors": get(payload, "authors"),
        "serializations": get(payload, "serializations"),
        "genres": get(payload, "genres"),
        "explicit_genres": get(payload, "explicit_genres"),
        "themes": get(payload, "themes"),
        "demographics": get(payload, "demographics"),
    })
}

/// `MangaFullResource`: the manga payload plus `relations` and `external`.
pub fn manga_full(payload: &Value) -> Value {
    let mut mapped = manga(payload);
    let object = mapped
        .as_object_mut()
        .expect("MangaResource always returns a JSON object");

    let related = get(payload, "related");
    object.insert("relations".to_string(), normalize_related(&related));
    object.insert("external".to_string(), get(payload, "external_links"));

    mapped
}

/// `MangaCharactersResource`: the stored document wraps characters under `characters`.
pub fn manga_characters(payload: &Value) -> Value {
    get(payload, "characters")
}

/// `MangaStatisticsResource`.
pub fn manga_statistics(payload: &Value) -> Value {
    json!({
        "reading": get(payload, "reading"),
        "completed": get(payload, "completed"),
        "on_hold": get(payload, "on_hold"),
        "dropped": get(payload, "dropped"),
        "plan_to_read": get(payload, "plan_to_read"),
        "total": get(payload, "total"),
        "scores": get(payload, "scores"),
    })
}

/// `MangaRelationsResource`: the stored document wraps relations under `related`.
pub fn manga_relations(payload: &Value) -> Value {
    get(payload, "related")
}

// ---------------------------------------------------------------------------
// Shared resources, manga-endpoint names.
// ---------------------------------------------------------------------------

/// `NewsResource` on `/manga/{id}/news`.
pub fn manga_news(payload: &Value) -> Value {
    misc::news(payload)
}

/// `ForumResource` on `/manga/{id}/forum`.
pub fn manga_forum(payload: &Value) -> Value {
    misc::forum(payload)
}

/// `PicturesResource` on `/manga/{id}/pictures`.
pub fn manga_pictures(payload: &Value) -> Value {
    misc::pictures(payload)
}

/// `MoreInfoResource` on `/manga/{id}/moreinfo`.
pub fn manga_more_info(payload: &Value) -> Value {
    misc::more_info(payload)
}

/// `RecommendationsResource` on `/manga/{id}/recommendations`.
pub fn manga_recommendations(payload: &Value) -> Value {
    misc::recommendations(payload)
}

/// `ExternalLinksResource` on `/manga/{id}/external`.
pub fn manga_external_links(payload: &Value) -> Value {
    misc::external_links(payload)
}

/// `UserUpdatesResource` on `/manga/{id}/userupdates`.
pub fn manga_user_updates(payload: &Value) -> Value {
    misc::results(payload)
}

/// `ReviewsResource` on `/manga/{id}/reviews`.
pub fn manga_reviews(payload: &Value) -> Value {
    misc::reviews(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors `database/factories/MangaFactory.php` plus related/external.
    const SAMPLE: &str = r#"{
    "mal_id": 2,
    "url": "https://myanimelist.net/manga/2/Berserk",
    "images": {
        "jpg": {
            "image_url": "https://cdn.myanimelist.net/images/manga/1/157897.jpg",
            "small_image_url": "https://cdn.myanimelist.net/images/manga/1/157897t.jpg",
            "large_image_url": "https://cdn.myanimelist.net/images/manga/1/157897l.jpg"
        },
        "webp": {
            "image_url": "https://cdn.myanimelist.net/images/manga/1/157897.webp",
            "small_image_url": "https://cdn.myanimelist.net/images/manga/1/157897t.webp",
            "large_image_url": "https://cdn.myanimelist.net/images/manga/1/157897l.webp"
        }
    },
    "approved": true,
    "titles": [{"type": "Default", "title": "Berserk"}],
    "title": "Berserk",
    "title_english": "Berserk",
    "title_japanese": "ベルセルク",
    "title_synonyms": [],
    "type": "Manga",
    "chapters": 364,
    "volumes": 41,
    "status": "Publishing",
    "publishing": true,
    "published": {
        "from": "1989-08-25T00:00:00+00:00",
        "to": null,
        "prop": {
            "from": {"day": 25, "month": 8, "year": 1989},
            "to": {"day": null, "month": null, "year": null}
        },
        "string": "Aug 25, 1989 to ?"
    },
    "score": 9.47,
    "scored_by": 425218,
    "rank": 1,
    "popularity": 2,
    "members": 1500000,
    "favorites": 200000,
    "synopsis": "Guts, a former mercenary now known as the Black Swordsman...",
    "background": "",
    "authors": [
        {"mal_id": 1867, "type": "people", "name": "Miura, Kentarou", "url": "https://myanimelist.net/people/1867/Kentarou_Miura"}
    ],
    "serializations": [
        {"mal_id": 48, "type": "manga", "name": "Young Animal", "url": "https://myanimelist.net/manga/magazine/48/Young_Animal"}
    ],
    "genres": [
        {"mal_id": 1, "type": "manga", "name": "Action", "url": "https://myanimelist.net/manga/genre/1/Action"}
    ],
    "explicit_genres": [],
    "themes": [],
    "demographics": [
        {"mal_id": 41, "type": "manga", "name": "Seinen", "url": "https://myanimelist.net/manga/genre/41/Seinen"}
    ],
    "related": {
        "Adaptation": [
            {"mal_id": 33, "type": "anime", "name": "Kenpuu Denki Berserk", "url": "https://myanimelist.net/anime/33/Kenpuu_Denki_Berserk"}
        ]
    },
    "external_links": [
        {"name": "Wikipedia", "url": "https://en.wikipedia.org/wiki/Berserk_(manga)"}
    ]
}"#;

    fn sample_manga() -> Value {
        serde_json::from_str(SAMPLE).expect("valid sample JSON")
    }

    #[test]
    fn manga_matches_full_php_key_set() {
        let mapped = manga(&sample_manga());

        const EXPECTED: &str = r#"{
    "mal_id": 2,
    "url": "https://myanimelist.net/manga/2/Berserk",
    "images": {
        "jpg": {
            "image_url": "https://cdn.myanimelist.net/images/manga/1/157897.jpg",
            "small_image_url": "https://cdn.myanimelist.net/images/manga/1/157897t.jpg",
            "large_image_url": "https://cdn.myanimelist.net/images/manga/1/157897l.jpg"
        },
        "webp": {
            "image_url": "https://cdn.myanimelist.net/images/manga/1/157897.webp",
            "small_image_url": "https://cdn.myanimelist.net/images/manga/1/157897t.webp",
            "large_image_url": "https://cdn.myanimelist.net/images/manga/1/157897l.webp"
        }
    },
    "approved": true,
    "titles": [{"type": "Default", "title": "Berserk"}],
    "title": "Berserk",
    "title_english": "Berserk",
    "title_japanese": "ベルセルク",
    "title_synonyms": [],
    "type": "Manga",
    "chapters": 364,
    "volumes": 41,
    "status": "Publishing",
    "publishing": true,
    "published": {
        "from": "1989-08-25T00:00:00+00:00",
        "to": null,
        "prop": {
            "from": {"day": 25, "month": 8, "year": 1989},
            "to": {"day": null, "month": null, "year": null}
        },
        "string": "Aug 25, 1989 to ?"
    },
    "score": 9.47,
    "scored": 9.47,
    "scored_by": 425218,
    "rank": 1,
    "popularity": 2,
    "members": 1500000,
    "favorites": 200000,
    "synopsis": "Guts, a former mercenary now known as the Black Swordsman...",
    "background": "",
    "authors": [
        {"mal_id": 1867, "type": "people", "name": "Miura, Kentarou", "url": "https://myanimelist.net/people/1867/Kentarou_Miura"}
    ],
    "serializations": [
        {"mal_id": 48, "type": "manga", "name": "Young Animal", "url": "https://myanimelist.net/manga/magazine/48/Young_Animal"}
    ],
    "genres": [
        {"mal_id": 1, "type": "manga", "name": "Action", "url": "https://myanimelist.net/manga/genre/1/Action"}
    ],
    "explicit_genres": [],
    "themes": [],
    "demographics": [
        {"mal_id": 41, "type": "manga", "name": "Seinen", "url": "https://myanimelist.net/manga/genre/41/Seinen"}
    ]
}"#;
        let expected: Value = serde_json::from_str(EXPECTED).expect("valid expected JSON");

        assert_eq!(mapped, expected);
        assert_eq!(mapped.as_object().unwrap().len(), 30);
    }

    #[test]
    fn manga_defaults_follow_php_null_coalescing() {
        let mapped = manga(&json!({}));
        assert_eq!(mapped["approved"], json!(true));
        assert_eq!(mapped["titles"], json!([]));
        assert_eq!(mapped["scored"], Value::Null);
        assert_eq!(mapped.as_object().unwrap().len(), 30);
    }

    #[test]
    fn manga_scored_duplicates_score_verbatim() {
        let mapped = manga(&json!({ "score": 8.12 }));
        assert_eq!(mapped["score"], json!(8.12));
        assert_eq!(mapped["scored"], json!(8.12));
    }

    #[test]
    fn manga_full_extends_with_relations_and_external() {
        let mapped = manga_full(&sample_manga());

        assert_eq!(
            mapped["relations"],
            json!([{
                "relation": "Adaptation",
                "entry": [
                    {"mal_id": 33, "type": "anime", "name": "Kenpuu Denki Berserk", "url": "https://myanimelist.net/anime/33/Kenpuu_Denki_Berserk"}
                ]
            }])
        );
        assert_eq!(
            mapped["external"],
            json!([{"name": "Wikipedia", "url": "https://en.wikipedia.org/wiki/Berserk_(manga)"}])
        );
        assert_eq!(mapped.as_object().unwrap().len(), 32);

        let mut base = mapped.clone();
        let object = base.as_object_mut().unwrap();
        for key in ["relations", "external"] {
            object.remove(key);
        }
        assert_eq!(base, manga(&sample_manga()));
    }

    #[test]
    fn manga_full_empty_related_is_object() {
        assert_eq!(manga_full(&json!({"related": {}}))["relations"], json!({}));
        assert_eq!(manga_full(&json!({}))["relations"], json!({}));
        let related = json!([{"relation": "Other", "entry": []}]);
        assert_eq!(
            manga_full(&json!({"related": related.clone()}))["relations"],
            related
        );
    }

    #[test]
    fn manga_characters_pass_through() {
        // tests/TestCase.php::givenDummyCharactersStaffData (manga variant)
        let payload = json!({
            "characters": [{
                "character": {
                    "mal_id": 3,
                    "url": "https://myanimelist.net/character/3/Jet_Black",
                    "images": {
                        "jpg": {"image_url": "https://cdn.myanimelist.net/images/characters/11/253723.jpg"},
                        "webp": {"image_url": "https://cdn.myanimelist.net/images/characters/11/253723.webp", "small_image_url": "https://cdn.myanimelist.net/images/characters/11/253723t.webp"}
                    },
                    "name": "Black, Jet"
                },
                "role": "Main"
            }]
        });
        assert_eq!(manga_characters(&payload), payload["characters"]);
        assert_eq!(manga_characters(&json!({})), Value::Null);
    }

    #[test]
    fn manga_statistics_maps_factory_document() {
        // tests/Integration/MangaControllerTest::testStats
        let payload = json!({
            "reading": 1293641,
            "completed": 37,
            "on_hold": 239382,
            "dropped": 164598,
            "plan_to_read": 195375,
            "total": 1893033,
            "scores": [
                {"score": 1, "votes": 9470, "percentage": 0.9},
                {"score": 2, "votes": 3363, "percentage": 0.3}
            ]
        });

        assert_eq!(
            manga_statistics(&payload),
            json!({
                "reading": 1293641,
                "completed": 37,
                "on_hold": 239382,
                "dropped": 164598,
                "plan_to_read": 195375,
                "total": 1893033,
                "scores": [
                    {"score": 1, "votes": 9470, "percentage": 0.9},
                    {"score": 2, "votes": 3363, "percentage": 0.3}
                ]
            })
        );
        let mapped = manga_statistics(&json!({}));
        assert_eq!(mapped.as_object().unwrap().len(), 7);
        assert_eq!(mapped["reading"], Value::Null);
    }

    #[test]
    fn manga_relations_resource_returns_related_verbatim() {
        // tests/Integration/MangaControllerTest::testMangaRelations
        let payload = json!({
            "related": [{
                "relation": "Other",
                "entry": [
                    {"mal_id": 793, "type": "manga", "name": "Wanted!", "url": "https://myanimelist.net/manga/793/Wanted"},
                    {"mal_id": 25146, "type": "manga", "name": "One Piece x Toriko", "url": "https://myanimelist.net/manga/25146/One_Piece_x_Toriko"}
                ]
            }]
        });
        assert_eq!(manga_relations(&payload), payload["related"]);
        assert_eq!(manga_relations(&json!({})), Value::Null);
    }

    #[test]
    fn manga_shared_resources_delegate() {
        let news = json!({
            "results": [{"mal_id": 60609964, "url": "https://myanimelist.net/news/60609964"}],
            "last_visible_page": 1,
            "has_next_page": false
        });
        assert_eq!(manga_news(&news), misc::news(&news));

        let forum = json!({"topics": [{"mal_id": 2022869, "comments": 7}]});
        assert_eq!(manga_forum(&forum), misc::forum(&forum));

        let pictures = json!({"pictures": [[{"jpg": {"image_url": "x"}}]]});
        assert_eq!(manga_pictures(&pictures), misc::pictures(&pictures));

        let moreinfo = json!({"moreinfo": "asd"});
        assert_eq!(manga_more_info(&moreinfo), misc::more_info(&moreinfo));

        let recommendations =
            json!({"recommendations": [{"entry": {"mal_id": 205}, "url": "u", "votes": 118}]});
        assert_eq!(
            manga_recommendations(&recommendations),
            misc::recommendations(&recommendations)
        );

        let links =
            json!({"external_links": [{"name": "Wikipedia", "url": "https://en.wikipedia.org"}]});
        assert_eq!(manga_external_links(&links), misc::external_links(&links));

        let updates = json!({
            "users": [{
                "user": {"username": "Mar-E"},
                "score": null,
                "status": "Watching",
                "volumes_read": 16,
                "volumes_total": 26,
                "chapters_read": 22,
                "chapters_total": 22,
                "date": "2023-01-31T22:34:00+00:00"
            }]
        });
        assert_eq!(manga_user_updates(&updates), misc::results(&updates));

        let reviews = json!({"results": [], "last_visible_page": 1, "has_next_page": false});
        assert_eq!(manga_reviews(&reviews), misc::reviews(&reviews));
    }
}
