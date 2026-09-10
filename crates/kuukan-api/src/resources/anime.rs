//! Anime response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/`:
//!
//! - [`anime`] — `AnimeResource`
//! - [`anime_full`] — `AnimeFullResource`
//! - [`anime_characters`] — `AnimeCharactersResource`
//! - [`anime_staff`] — `AnimeStaffResource`
//! - [`anime_episodes`] — `AnimeEpisodesResource`
//! - [`anime_episode`] — `AnimeEpisodeResource`
//! - [`anime_videos`] — `AnimeVideosResource`
//! - [`anime_statistics`] — `AnimeStatisticsResource`
//! - [`anime_relations`] — `AnimeRelationsResource`
//! - [`anime_themes`] — `AnimeThemesResource`
//!
//! Shared resources used by anime endpoints are re-exported as `anime_*`
//! wrappers around the canonical mappers in [`crate::resources::misc`]:
//! `NewsResource`, `ForumResource`, `PicturesResource`, `MoreInfoResource`,
//! `RecommendationsResource`, `StreamingLinksResource`, `ExternalLinksResource`,
//! `UserUpdatesResource` and `ReviewsResource`.
//!
//! Every mapper emits the full PHP key set (nulls included) on the stored,
//! JMS snake_case document. `related` is normalized with
//! [`kuukan_core::util::normalize_related`] for the main/full payload, matching
//! `HttpHelper::serializeEmptyObjectsControllerLevel` (`{}` for empty).

use kuukan_core::util::normalize_related;
use serde_json::{json, Value};

use crate::resources::misc::{self, get, get_or};

/// `AnimeResource`.
pub fn anime(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "trailer": get(payload, "trailer"),
        "approved": get_or(payload, "approved", json!(true)),
        "titles": get_or(payload, "titles", json!([])),
        "title": get(payload, "title"),
        "title_english": get(payload, "title_english"),
        "title_japanese": get(payload, "title_japanese"),
        "title_synonyms": get(payload, "title_synonyms"),
        "type": get(payload, "type"),
        "source": get(payload, "source"),
        "episodes": get(payload, "episodes"),
        "status": get(payload, "status"),
        "airing": get(payload, "airing"),
        "aired": get(payload, "aired"),
        "duration": get(payload, "duration"),
        "rating": get(payload, "rating"),
        "score": get(payload, "score"),
        "scored_by": get(payload, "scored_by"),
        "rank": get(payload, "rank"),
        "popularity": get(payload, "popularity"),
        "members": get(payload, "members"),
        "favorites": get(payload, "favorites"),
        "synopsis": get(payload, "synopsis"),
        "background": get(payload, "background"),
        "season": get(payload, "season"),
        "year": get(payload, "year"),
        "broadcast": get(payload, "broadcast"),
        "producers": get(payload, "producers"),
        "licensors": get(payload, "licensors"),
        "studios": get(payload, "studios"),
        "genres": get(payload, "genres"),
        "explicit_genres": get(payload, "explicit_genres"),
        "themes": get(payload, "themes"),
        "demographics": get(payload, "demographics"),
    })
}

/// `AnimeFullResource`: the anime payload plus `relations`, `theme`,
/// `external` and `streaming`.
pub fn anime_full(payload: &Value) -> Value {
    let mut mapped = anime(payload);
    let object = mapped
        .as_object_mut()
        .expect("AnimeResource always returns a JSON object");

    let related = get(payload, "related");
    object.insert("relations".to_string(), normalize_related(&related));
    object.insert(
        "theme".to_string(),
        json!({
            "openings": get(payload, "opening_themes"),
            "endings": get(payload, "ending_themes"),
        }),
    );
    object.insert("external".to_string(), get(payload, "external_links"));
    object.insert("streaming".to_string(), get(payload, "streaming_links"));

    mapped
}

/// `AnimeCharactersResource`: the stored document wraps characters under `characters`.
pub fn anime_characters(payload: &Value) -> Value {
    get(payload, "characters")
}

/// `AnimeStaffResource`: the stored document wraps staff under `staff`.
pub fn anime_staff(payload: &Value) -> Value {
    get(payload, "staff")
}

/// `AnimeEpisodesResource` (also used by `/anime/{id}/videos/episodes`).
pub fn anime_episodes(payload: &Value) -> Value {
    json!({
        "pagination": misc::list_pagination(payload),
        "data": get(payload, "results"),
    })
}

/// `AnimeEpisodeResource`.
pub fn anime_episode(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "title": get(payload, "title"),
        "title_japanese": get(payload, "title_japanese"),
        "title_romanji": get(payload, "title_romanji"),
        "duration": get(payload, "duration"),
        "aired": get(payload, "aired"),
        "filler": get(payload, "filler"),
        "recap": get(payload, "recap"),
        "synopsis": get(payload, "synopsis"),
    })
}

/// `AnimeVideosResource`.
pub fn anime_videos(payload: &Value) -> Value {
    json!({
        "promo": get(payload, "promo"),
        "episodes": get(payload, "episodes"),
        "music_videos": get(payload, "music_videos"),
    })
}

/// `AnimeStatisticsResource`.
pub fn anime_statistics(payload: &Value) -> Value {
    json!({
        "watching": get(payload, "watching"),
        "completed": get(payload, "completed"),
        "on_hold": get(payload, "on_hold"),
        "dropped": get(payload, "dropped"),
        "plan_to_watch": get(payload, "plan_to_watch"),
        "total": get(payload, "total"),
        "scores": get(payload, "scores"),
    })
}

/// `AnimeRelationsResource`: the stored document wraps relations under `related`.
pub fn anime_relations(payload: &Value) -> Value {
    get(payload, "related")
}

/// `AnimeThemesResource`.
pub fn anime_themes(payload: &Value) -> Value {
    json!({
        "openings": get_or(payload, "opening_themes", json!([])),
        "endings": get_or(payload, "ending_themes", json!([])),
    })
}

// ---------------------------------------------------------------------------
// Shared resources, anime-endpoint names.
// ---------------------------------------------------------------------------

/// `NewsResource` on `/anime/{id}/news`.
pub fn anime_news(payload: &Value) -> Value {
    misc::news(payload)
}

/// `ForumResource` on `/anime/{id}/forum`.
pub fn anime_forum(payload: &Value) -> Value {
    misc::forum(payload)
}

/// `PicturesResource` on `/anime/{id}/pictures`.
pub fn anime_pictures(payload: &Value) -> Value {
    misc::pictures(payload)
}

/// `MoreInfoResource` on `/anime/{id}/moreinfo`.
pub fn anime_more_info(payload: &Value) -> Value {
    misc::more_info(payload)
}

/// `RecommendationsResource` on `/anime/{id}/recommendations`.
pub fn anime_recommendations(payload: &Value) -> Value {
    misc::recommendations(payload)
}

/// `StreamingLinksResource` on `/anime/{id}/streaming`.
pub fn anime_streaming_links(payload: &Value) -> Value {
    misc::streaming_links(payload)
}

/// `ExternalLinksResource` on `/anime/{id}/external`.
pub fn anime_external_links(payload: &Value) -> Value {
    misc::external_links(payload)
}

/// `UserUpdatesResource` on `/anime/{id}/userupdates`.
pub fn anime_user_updates(payload: &Value) -> Value {
    misc::results(payload)
}

/// `ReviewsResource` on `/anime/{id}/reviews`.
pub fn anime_reviews(payload: &Value) -> Value {
    misc::reviews(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors `database/factories/AnimeFactory.php` plus the related/themes/
    /// links fields the real scraper stores.
    const SAMPLE: &str = r#"{
    "mal_id": 1,
    "url": "https://myanimelist.net/anime/1/Cowboy_Bebop",
    "images": {
        "jpg": {
            "image_url": "https://cdn.myanimelist.net/images/anime/4/19644.jpg",
            "small_image_url": "https://cdn.myanimelist.net/images/anime/4/19644t.jpg",
            "large_image_url": "https://cdn.myanimelist.net/images/anime/4/19644l.jpg"
        },
        "webp": {
            "image_url": "https://cdn.myanimelist.net/images/anime/4/19644.webp",
            "small_image_url": "https://cdn.myanimelist.net/images/anime/4/19644t.webp",
            "large_image_url": "https://cdn.myanimelist.net/images/anime/4/19644l.webp"
        }
    },
    "trailer": {
        "youtube_id": "qig4KOK2R2g",
        "url": "https://www.youtube.com/watch?v=qig4KOK2R2g",
        "embed_url": "https://www.youtube.com/embed/qig4KOK2R2g?enablejsapi=1&wmode=opaque&autoplay=1",
        "images": {
            "image_url": "https://img.youtube.com/vi/qig4KOK2R2g/default.jpg",
            "small_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/sddefault.jpg",
            "medium_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/mqdefault.jpg",
            "large_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/hqdefault.jpg",
            "maximum_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/maxresdefault.jpg"
        }
    },
    "approved": true,
    "titles": [{"type": "Default", "title": "Cowboy Bebop"}],
    "title": "Cowboy Bebop",
    "title_english": "Cowboy Bebop",
    "title_japanese": "カウボーイビバップ",
    "title_synonyms": [],
    "type": "TV",
    "source": "Original",
    "episodes": 26,
    "status": "Finished Airing",
    "airing": false,
    "aired": {
        "from": "1998-04-03T00:00:00+00:00",
        "to": "1999-04-24T00:00:00+00:00",
        "prop": {
            "from": {"day": 3, "month": 4, "year": 1998},
            "to": {"day": 24, "month": 4, "year": 1999}
        },
        "string": "Apr 3, 1998 to Apr 24, 1999"
    },
    "duration": "24 min per ep",
    "rating": "R - 17+ (violence & profanity)",
    "score": 8.75,
    "scored_by": 808331,
    "rank": 45,
    "popularity": 43,
    "members": 2001234,
    "favorites": 100456,
    "synopsis": "In the year 2071, humanity has colonized several of the planets.",
    "background": "",
    "season": "spring",
    "year": 1998,
    "broadcast": {
        "day": "Saturdays",
        "time": "01:00",
        "timezone": "Asia/Tokyo",
        "string": "Saturdays at 01:00 (JST)"
    },
    "producers": [
        {"mal_id": 16, "type": "anime", "name": "TV Tokyo", "url": "https://myanimelist.net/anime/producer/16/TV_Tokyo"}
    ],
    "licensors": [
        {"mal_id": 102, "type": "anime", "name": "Funimation", "url": "https://myanimelist.net/anime/producer/102/Funimation"}
    ],
    "studios": [
        {"mal_id": 14, "type": "anime", "name": "Sunrise", "url": "https://myanimelist.net/anime/producer/14/Sunrise"}
    ],
    "genres": [
        {"mal_id": 1, "type": "anime", "name": "Action", "url": "https://myanimelist.net/anime/genre/1/Action"}
    ],
    "explicit_genres": [],
    "themes": [
        {"mal_id": 50, "type": "anime", "name": "Adult Cast", "url": "https://myanimelist.net/anime/genre/50/Adult_Cast"}
    ],
    "demographics": [
        {"mal_id": 27, "type": "anime", "name": "Shounen", "url": "https://myanimelist.net/anime/genre/27/Shounen"}
    ],
    "related": {
        "Adaptation": [
            {"mal_id": 173, "type": "manga", "name": "Cowboy Bebop", "url": "https://myanimelist.net/manga/173/Cowboy_Bebop"}
        ]
    },
    "opening_themes": ["Tank! by Seatbelts"],
    "ending_themes": ["The Real Folk Blues by Mai Yamane"],
    "external_links": [
        {"name": "Wikipedia", "url": "https://en.wikipedia.org/wiki/Cowboy_Bebop"}
    ],
    "streaming_links": [
        {"name": "Crunchyroll", "url": "https://www.crunchyroll.com/cowboy-bebop"}
    ]
}"#;

    fn sample_anime() -> Value {
        serde_json::from_str(SAMPLE).expect("valid sample JSON")
    }

    #[test]
    fn anime_matches_full_php_key_set() {
        let mapped = anime(&sample_anime());

        const EXPECTED: &str = r#"{
    "mal_id": 1,
    "url": "https://myanimelist.net/anime/1/Cowboy_Bebop",
    "images": {
        "jpg": {
            "image_url": "https://cdn.myanimelist.net/images/anime/4/19644.jpg",
            "small_image_url": "https://cdn.myanimelist.net/images/anime/4/19644t.jpg",
            "large_image_url": "https://cdn.myanimelist.net/images/anime/4/19644l.jpg"
        },
        "webp": {
            "image_url": "https://cdn.myanimelist.net/images/anime/4/19644.webp",
            "small_image_url": "https://cdn.myanimelist.net/images/anime/4/19644t.webp",
            "large_image_url": "https://cdn.myanimelist.net/images/anime/4/19644l.webp"
        }
    },
    "trailer": {
        "youtube_id": "qig4KOK2R2g",
        "url": "https://www.youtube.com/watch?v=qig4KOK2R2g",
        "embed_url": "https://www.youtube.com/embed/qig4KOK2R2g?enablejsapi=1&wmode=opaque&autoplay=1",
        "images": {
            "image_url": "https://img.youtube.com/vi/qig4KOK2R2g/default.jpg",
            "small_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/sddefault.jpg",
            "medium_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/mqdefault.jpg",
            "large_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/hqdefault.jpg",
            "maximum_image_url": "https://img.youtube.com/vi/qig4KOK2R2g/maxresdefault.jpg"
        }
    },
    "approved": true,
    "titles": [{"type": "Default", "title": "Cowboy Bebop"}],
    "title": "Cowboy Bebop",
    "title_english": "Cowboy Bebop",
    "title_japanese": "カウボーイビバップ",
    "title_synonyms": [],
    "type": "TV",
    "source": "Original",
    "episodes": 26,
    "status": "Finished Airing",
    "airing": false,
    "aired": {
        "from": "1998-04-03T00:00:00+00:00",
        "to": "1999-04-24T00:00:00+00:00",
        "prop": {
            "from": {"day": 3, "month": 4, "year": 1998},
            "to": {"day": 24, "month": 4, "year": 1999}
        },
        "string": "Apr 3, 1998 to Apr 24, 1999"
    },
    "duration": "24 min per ep",
    "rating": "R - 17+ (violence & profanity)",
    "score": 8.75,
    "scored_by": 808331,
    "rank": 45,
    "popularity": 43,
    "members": 2001234,
    "favorites": 100456,
    "synopsis": "In the year 2071, humanity has colonized several of the planets.",
    "background": "",
    "season": "spring",
    "year": 1998,
    "broadcast": {
        "day": "Saturdays",
        "time": "01:00",
        "timezone": "Asia/Tokyo",
        "string": "Saturdays at 01:00 (JST)"
    },
    "producers": [
        {"mal_id": 16, "type": "anime", "name": "TV Tokyo", "url": "https://myanimelist.net/anime/producer/16/TV_Tokyo"}
    ],
    "licensors": [
        {"mal_id": 102, "type": "anime", "name": "Funimation", "url": "https://myanimelist.net/anime/producer/102/Funimation"}
    ],
    "studios": [
        {"mal_id": 14, "type": "anime", "name": "Sunrise", "url": "https://myanimelist.net/anime/producer/14/Sunrise"}
    ],
    "genres": [
        {"mal_id": 1, "type": "anime", "name": "Action", "url": "https://myanimelist.net/anime/genre/1/Action"}
    ],
    "explicit_genres": [],
    "themes": [
        {"mal_id": 50, "type": "anime", "name": "Adult Cast", "url": "https://myanimelist.net/anime/genre/50/Adult_Cast"}
    ],
    "demographics": [
        {"mal_id": 27, "type": "anime", "name": "Shounen", "url": "https://myanimelist.net/anime/genre/27/Shounen"}
    ]
}"#;
        let expected: Value = serde_json::from_str(EXPECTED).expect("valid expected JSON");

        assert_eq!(mapped, expected);
        assert_eq!(mapped.as_object().unwrap().len(), 36);
        // deprecated fields are present
        for key in ["title", "title_english", "title_japanese", "title_synonyms"] {
            assert!(
                mapped.as_object().unwrap().contains_key(key),
                "{key} missing"
            );
        }
    }

    #[test]
    fn anime_defaults_follow_php_null_coalescing() {
        let mapped = anime(&json!({}));
        assert_eq!(mapped["approved"], json!(true));
        assert_eq!(mapped["titles"], json!([]));
        assert_eq!(mapped["mal_id"], Value::Null);
        assert_eq!(mapped.as_object().unwrap().len(), 36);
    }

    #[test]
    fn anime_full_extends_with_relations_theme_links() {
        let mapped = anime_full(&sample_anime());

        assert_eq!(
            mapped["relations"],
            json!([{
                "relation": "Adaptation",
                "entry": [
                    {"mal_id": 173, "type": "manga", "name": "Cowboy Bebop", "url": "https://myanimelist.net/manga/173/Cowboy_Bebop"}
                ]
            }])
        );
        assert_eq!(
            mapped["theme"],
            json!({
                "openings": ["Tank! by Seatbelts"],
                "endings": ["The Real Folk Blues by Mai Yamane"]
            })
        );
        assert_eq!(
            mapped["external"],
            json!([{"name": "Wikipedia", "url": "https://en.wikipedia.org/wiki/Cowboy_Bebop"}])
        );
        assert_eq!(
            mapped["streaming"],
            json!([{"name": "Crunchyroll", "url": "https://www.crunchyroll.com/cowboy-bebop"}])
        );
        assert_eq!(mapped.as_object().unwrap().len(), 40);
        // base payload unchanged
        let mut base = mapped.clone();
        let object = base.as_object_mut().unwrap();
        for key in ["relations", "theme", "external", "streaming"] {
            object.remove(key);
        }
        assert_eq!(base, anime(&sample_anime()));
    }

    #[test]
    fn anime_full_empty_related_is_object() {
        // `{}` stays `{}` (HttpHelper::serializeEmptyObjectsControllerLevel)
        let mapped = anime_full(&json!({ "related": {} }));
        assert_eq!(mapped["relations"], json!({}));

        // missing related still renders as {} for full payloads (normalize_related)
        let mapped = anime_full(&json!({}));
        assert_eq!(mapped["relations"], json!({}));

        // a document already normalized to a list is passed through untouched
        let related = json!([{"relation": "Adaptation", "entry": []}]);
        let mapped = anime_full(&json!({ "related": related }));
        assert_eq!(mapped["relations"], related);
    }

    #[test]
    fn anime_characters_and_staff_pass_through() {
        // tests/TestCase.php::givenDummyCharactersStaffData
        let payload = json!({
            "characters": [{
                "character": {
                    "mal_id": 3,
                    "url": "https://myanimelist.net/character/3/Jet_Black",
                    "images": {
                        "jpg": {
                            "image_url": "https://cdn.myanimelist.net/images/characters/11/253723.jpg",
                            "small_image_url": "https://cdn.myanimelist.net/images/characters/11/253723t.jpg"
                        },
                        "webp": {
                            "image_url": "https://cdn.myanimelist.net/images/characters/11/253723.webp",
                            "small_image_url": "https://cdn.myanimelist.net/images/characters/11/253723t.webp"
                        }
                    },
                    "name": "Black, Jet"
                },
                "role": "Main",
                "favorites": 1,
                "voice_actors": [{
                    "person": {
                        "mal_id": 357,
                        "url": "https://myanimelist.net/people/357/Unshou_Ishizuk",
                        "images": {
                            "jpg": {"image_url": "https://cdn.myanimelist.net/images/voiceactors/2/17135.jpg"}
                        },
                        "name": "Ishizuka, Unshou"
                    },
                    "language": "Japanese"
                }]
            }],
            "staff": [{
                "person": {
                    "mal_id": 40009,
                    "url": "https://myanimelist.net/people/40009/Yutaka_Maseba",
                    "images": {
                        "jpg": {"image_url": "https://cdn.myanimelist.net/images/voiceactors/3/40216.jpg"}
                    },
                    "name": "Maseba, Yutaka"
                },
                "positions": ["Producer"]
            }]
        });

        assert_eq!(anime_characters(&payload), payload["characters"]);
        assert_eq!(anime_staff(&payload), payload["staff"]);
        assert_eq!(anime_characters(&json!({})), Value::Null);
        assert_eq!(anime_staff(&json!({})), Value::Null);
    }

    #[test]
    fn anime_episodes_envelope_matches_results_resource() {
        // /anime/{id}/episodes uses the base ResultsResource in PHP
        let payload = json!({
            "results": [{
                "mal_id": 301,
                "url": "https://myanimelist.net/anime/516/Keroro_Gunsou/episode/301",
                "title": "Tamama, Exiled from the Nishizawa House, Sir? / Momoka: A Chocolate ",
                "title_japanese": null,
                "title_romanji": null,
                "aired": null,
                "filler": false,
                "recap": false,
                "score": 4.5,
                "forum_url": null
            }],
            "last_visible_page": 2,
            "has_next_page": true,
        });

        let mapped = anime_episodes(&payload);
        assert_eq!(
            mapped["pagination"],
            json!({"last_visible_page": 2, "has_next_page": true})
        );
        assert_eq!(mapped["data"], payload["results"]);
        assert_eq!(mapped, misc::results(&payload));

        let mapped = anime_episodes(&json!({}));
        assert_eq!(
            mapped,
            json!({
                "pagination": {"last_visible_page": 1, "has_next_page": false},
                "data": null
            })
        );
    }

    #[test]
    fn anime_episode_maps_ten_php_keys() {
        // tests/Integration/AnimeControllerTest::testEpisode
        let payload = json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/anime/21/One_Piece/episode/1",
            "title": "I'm Luffy! The Man Who's Gonna Be King of the Pirates!",
            "title_japanese": "俺はルフィ!海賊王になる男だ!",
            "title_romanji": "Ore wa Luffy! Kaizoku Ou ni Naru Otoko Da!",
            "duration": 1475,
            "aired": "1999-10-20T00:00:00+09:00",
            "filler": false,
            "recap": false,
            "synopsis": "The series begins with an attack",
            // raw parser extras that the PHP resource drops
            "score": 4.5,
            "forum_url": null,
        });

        assert_eq!(
            anime_episode(&payload),
            json!({
                "mal_id": 1,
                "url": "https://myanimelist.net/anime/21/One_Piece/episode/1",
                "title": "I'm Luffy! The Man Who's Gonna Be King of the Pirates!",
                "title_japanese": "俺はルフィ!海賊王になる男だ!",
                "title_romanji": "Ore wa Luffy! Kaizoku Ou ni Naru Otoko Da!",
                "duration": 1475,
                "aired": "1999-10-20T00:00:00+09:00",
                "filler": false,
                "recap": false,
                "synopsis": "The series begins with an attack",
            })
        );

        // duration/synopsis resolve to null when the source document has neither
        let mapped = anime_episode(&json!({"mal_id": 5, "title": "x"}));
        assert_eq!(mapped["duration"], Value::Null);
        assert_eq!(mapped["synopsis"], Value::Null);
        assert_eq!(mapped.as_object().unwrap().len(), 10);
    }

    #[test]
    fn anime_videos_passes_three_sections() {
        // tests/Integration/AnimeControllerTest::testVideos
        let payload = json!({
            "promo": [{"title": "PV Blu-ray Box version", "trailer": {"youtube_id": "qig4KOK2R2g"}}],
            "episodes": [{
                "mal_id": 26,
                "title": "The Real Folk Blues (part 2)",
                "episode": "Episode 26",
                "url": "https://myanimelist.net/anime/1/Cowboy_Bebop/episode/26",
                "images": {"jpg": {"image_url": "https://img1.ak.crunchyroll.com/i/spire1-tmb/191b230426f0b0e6568b4ca6edab47321473136587_large.jpg"}}
            }],
            "music_videos": [{
                "title": "OP 1 (Artist ver.)",
                "video": {"youtube_id": "wbaILDE7Dco"},
                "meta": {"title": "\"heavenly blue\"", "author": "Kalafina"}
            }]
        });

        assert_eq!(
            anime_videos(&payload),
            json!({
                "promo": payload["promo"],
                "episodes": payload["episodes"],
                "music_videos": payload["music_videos"],
            })
        );
        assert_eq!(
            anime_videos(&json!({})),
            json!({"promo": null, "episodes": null, "music_videos": null})
        );
    }

    #[test]
    fn anime_statistics_maps_factory_document() {
        // tests/Integration/AnimeControllerTest::testStats
        let payload = json!({
            "watching": 1293641,
            "completed": 37,
            "on_hold": 239382,
            "dropped": 164598,
            "plan_to_watch": 195375,
            "total": 1893033,
            "scores": [
                {"score": 1, "votes": 9470, "percentage": 0.9},
                {"score": 2, "votes": 3363, "percentage": 0.3}
            ]
        });

        assert_eq!(
            anime_statistics(&payload),
            json!({
                "watching": 1293641,
                "completed": 37,
                "on_hold": 239382,
                "dropped": 164598,
                "plan_to_watch": 195375,
                "total": 1893033,
                "scores": [
                    {"score": 1, "votes": 9470, "percentage": 0.9},
                    {"score": 2, "votes": 3363, "percentage": 0.3}
                ]
            })
        );

        let mapped = anime_statistics(&json!({}));
        assert_eq!(mapped.as_object().unwrap().len(), 7);
        assert_eq!(mapped["scores"], Value::Null);
    }

    #[test]
    fn anime_relations_resource_returns_related_verbatim() {
        // tests/Integration/AnimeControllerTest::testAnimeRelations
        let payload = json!({
            "related": [{
                "relation": "Adaptation",
                "entry": [
                    {"mal_id": 173, "type": "manga", "name": "Cowboy Bebop", "url": "https://myanimelist.net/manga/173/Cowboy_Bebop"},
                    {"mal_id": 174, "type": "manga", "name": "Shooting Star Bebop: Cowboy Bebop", "url": "https://myanimelist.net/manga/174/Shooting_Star_Bebop__Cowboy_Bebop"}
                ]
            }]
        });
        assert_eq!(anime_relations(&payload), payload["related"]);
        assert_eq!(anime_relations(&json!({})), Value::Null);
    }

    #[test]
    fn anime_themes_defaults_to_empty_arrays() {
        let payload = json!({
            "opening_themes": ["OP 1: Tank!"],
            "ending_themes": ["ED 1: The Real Folk Blues"]
        });
        assert_eq!(
            anime_themes(&payload),
            json!({"openings": ["OP 1: Tank!"], "endings": ["ED 1: The Real Folk Blues"]})
        );
        assert_eq!(
            anime_themes(&json!({})),
            json!({"openings": [], "endings": []})
        );
        assert_eq!(
            anime_themes(&json!({"opening_themes": null, "ending_themes": null})),
            json!({"openings": [], "endings": []})
        );
    }

    #[test]
    fn anime_more_info_news_forum_pictures_delegate_to_shared_mappers() {
        let moreinfo = json!({"moreinfo": "asd"});
        assert_eq!(anime_more_info(&moreinfo), misc::more_info(&moreinfo));

        let news = json!({
            "results": [{"mal_id": 60609964, "url": "https://myanimelist.net/news/60609964"}],
            "last_visible_page": 1,
            "has_next_page": false
        });
        assert_eq!(anime_news(&news), misc::news(&news));

        let forum = json!({"topics": [{"mal_id": 2022869, "comments": 7}]});
        assert_eq!(anime_forum(&forum), misc::forum(&forum));

        let pictures = json!({"pictures": [[{"jpg": {"image_url": "x"}}]]});
        assert_eq!(anime_pictures(&pictures), misc::pictures(&pictures));

        let recommendations =
            json!({"recommendations": [{"entry": {"mal_id": 205}, "url": "u", "votes": 118}]});
        assert_eq!(
            anime_recommendations(&recommendations),
            misc::recommendations(&recommendations)
        );

        let links = json!({
            "external_links": [{"name": "Wikipedia", "url": "https://en.wikipedia.org"}],
            "streaming_links": [{"name": "Crunchyroll", "url": "https://crunchyroll.com"}]
        });
        assert_eq!(anime_external_links(&links), misc::external_links(&links));
        assert_eq!(anime_streaming_links(&links), misc::streaming_links(&links));

        let updates = json!({"users": [{"user": {"username": "Mar-E"}, "score": null}]});
        assert_eq!(anime_user_updates(&updates), misc::results(&updates));

        let reviews = json!({"results": [], "last_visible_page": 1, "has_next_page": false});
        assert_eq!(anime_reviews(&reviews), misc::reviews(&reviews));
    }
}
