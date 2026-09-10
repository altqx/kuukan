//! Person response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/PersonResource.php`,
//! `PersonFullResource.php`, `PersonAnimeResource.php` (+
//! `PersonAnimeCollection`), `PersonMangaResource.php` (+
//! `PersonMangaCollection`), `PersonVoiceResource.php` (+
//! `PersonVoicesCollection`) and `PersonCollection.php` (search).
//!
//! `/people/{id}/pictures` uses the shared `PicturesResource`, re-exported
//! here from `resources::character`.

use crate::resources::misc::{get, get_first};
use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::{json, Value};

pub use super::character::pictures;

/// `PersonResource` reads `$this->favorites`, a model accessor over the
/// stored `member_favorites` column; JMS documents carry `member_favorites`
/// while model-shaped documents (Eloquent `toArray`) carry `favorites`.
fn favorites(payload: &Value) -> Value {
    get_first(payload, &["favorites", "member_favorites"])
}

/// `PersonResource::toArray()`.
///
/// ```text
/// {mal_id, url, website_url, images, name, given_name, family_name,
///  alternate_names, birthday, favorites, about}
/// ```
pub fn person(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "website_url": get(payload, "website_url"),
        "images": get(payload, "images"),
        "name": get(payload, "name"),
        "given_name": get(payload, "given_name"),
        "family_name": get(payload, "family_name"),
        "alternate_names": get(payload, "alternate_names"),
        "birthday": get(payload, "birthday"),
        "favorites": favorites(payload),
        "about": get(payload, "about"),
    })
}

/// `PersonFullResource::toArray()`.
///
/// `anime`, `manga` and `voices` are the raw `anime_staff_positions`,
/// `published_manga` and `voice_acting_roles` lists (the PHP resource does
/// not map them through the per-item resources).
pub fn person_full(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "website_url": get(payload, "website_url"),
        "images": get(payload, "images"),
        "name": get(payload, "name"),
        "given_name": get(payload, "given_name"),
        "family_name": get(payload, "family_name"),
        "alternate_names": get(payload, "alternate_names"),
        "birthday": get(payload, "birthday"),
        "favorites": favorites(payload),
        "about": get(payload, "about"),
        "anime": get(payload, "anime_staff_positions"),
        "manga": get(payload, "published_manga"),
        "voices": get(payload, "voice_acting_roles"),
    })
}

/// `PersonAnimeResource::toArray()` for one `anime_staff_positions` item.
pub fn person_anime(item: &Value) -> Value {
    let anime = item.get("anime").cloned().unwrap_or(Value::Null);
    json!({
        "position": get(item, "position"),
        "anime": {
            "mal_id": get(&anime, "mal_id"),
            "url": get(&anime, "url"),
            "images": get(&anime, "images"),
            "title": get(&anime, "title"),
        },
    })
}

/// `PersonAnimeCollection` item mapping.
pub fn person_anime_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(person_anime).collect()
}

/// `PersonMangaResource::toArray()` for one `published_manga` item.
pub fn person_manga(item: &Value) -> Value {
    let manga = item.get("manga").cloned().unwrap_or(Value::Null);
    json!({
        "position": get(item, "position"),
        "manga": {
            "mal_id": get(&manga, "mal_id"),
            "url": get(&manga, "url"),
            "images": get(&manga, "images"),
            "title": get(&manga, "title"),
        },
    })
}

/// `PersonMangaCollection` item mapping.
pub fn person_manga_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(person_manga).collect()
}

/// `PersonVoiceResource::toArray()` for one `voice_acting_roles` item.
pub fn person_voice(item: &Value) -> Value {
    let anime = item.get("anime").cloned().unwrap_or(Value::Null);
    let character = item.get("character").cloned().unwrap_or(Value::Null);
    json!({
        "role": get(item, "role"),
        "anime": {
            "mal_id": get(&anime, "mal_id"),
            "url": get(&anime, "url"),
            "images": get(&anime, "images"),
            "title": get(&anime, "title"),
        },
        "character": {
            "mal_id": get(&character, "mal_id"),
            "url": get(&character, "url"),
            "images": get(&character, "images"),
            "name": get(&character, "name"),
        },
    })
}

/// `PersonVoicesCollection` item mapping.
pub fn person_voices_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(person_voice).collect()
}

/// Alias of [`pictures`] for `/people/{id}/pictures`.
pub fn person_pictures(payload: &Value) -> Value {
    pictures(payload)
}

/// `PersonCollection` item mapping (search results; `$collects` is
/// `PersonResource`).
pub fn person_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(person).collect()
}

/// Envelope for `PersonCollection`: `{"pagination": {...}, "data": [...]}`.
pub fn person_search_response(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, person_collection(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn assert_object_keys(value: &Value, expected: &[&str]) {
        let object = value.as_object().expect("expected a JSON object");
        let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = expected.iter().copied().collect();
        assert_eq!(actual, expected, "unexpected key set in {value}");
    }

    fn images() -> Value {
        json!({
            "jpg": {
                "image_url": "https://cdn.myanimelist.net/images/voiceactors/1/55486.jpg"
            }
        })
    }

    fn jms_person() -> Value {
        json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/people/1/Kouichi_Yamadera",
            "website_url": "https://webiste.example",
            "images": images(),
            "name": "Yamadera, Kouichi",
            "given_name": "Kouichi",
            "family_name": "Yamadera",
            "alternate_names": ["山寺宏一"],
            "birthday": "1961-06-17T00:00:00+00:00",
            "member_favorites": 1413,
            "about": "test",
            "anime_staff_positions": [],
            "published_manga": [],
            "voice_acting_roles": []
        })
    }

    #[test]
    fn person_maps_all_fields() {
        let mapped = person(&jms_person());
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "website_url",
                "images",
                "name",
                "given_name",
                "family_name",
                "alternate_names",
                "birthday",
                "favorites",
                "about",
            ],
        );
        assert_eq!(mapped["mal_id"], json!(1));
        assert_eq!(mapped["website_url"], json!("https://webiste.example"));
        assert_eq!(mapped["given_name"], json!("Kouichi"));
        assert_eq!(mapped["family_name"], json!("Yamadera"));
        assert_eq!(mapped["alternate_names"], json!(["山寺宏一"]));
        assert_eq!(mapped["birthday"], json!("1961-06-17T00:00:00+00:00"));
        assert_eq!(mapped["favorites"], json!(1413));
    }

    #[test]
    fn person_reads_model_shaped_favorites() {
        let mut document = jms_person();
        let map = document.as_object_mut().unwrap();
        map.remove("member_favorites");
        map.insert("favorites".to_string(), json!(99));
        assert_eq!(person(&document)["favorites"], json!(99));
    }

    #[test]
    fn person_emits_nulls_for_missing_keys() {
        assert_eq!(
            person(&json!({})),
            json!({
                "mal_id": null,
                "url": null,
                "website_url": null,
                "images": null,
                "name": null,
                "given_name": null,
                "family_name": null,
                "alternate_names": null,
                "birthday": null,
                "favorites": null,
                "about": null,
            })
        );
    }

    #[test]
    fn person_full_inlines_raw_lists() {
        let document = json!({
            "mal_id": 1,
            "member_favorites": 5,
            "anime_staff_positions": [{
                "position": "Theme Song Performance",
                "anime": {"mal_id": 3080, "url": "https://myanimelist.net/anime/3080", "images": null, "title": "Anime Tenchou"}
            }],
            "published_manga": [{
                "position": "Art",
                "manga": {"mal_id": 3080, "url": "https://myanimelist.net/manga/3080", "images": null, "title": "Anime Tenchou"}
            }],
            "voice_acting_roles": [{
                "role": "Main",
                "anime": {"mal_id": 1, "url": "https://myanimelist.net/anime/1", "images": null, "title": "Cowboy Bebop"},
                "character": {"mal_id": 1, "url": "https://myanimelist.net/character/1", "images": null, "name": "Spike"}
            }]
        });

        let mapped = person_full(&document);
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "website_url",
                "images",
                "name",
                "given_name",
                "family_name",
                "alternate_names",
                "birthday",
                "favorites",
                "about",
                "anime",
                "manga",
                "voices",
            ],
        );
        assert_eq!(mapped["anime"], document["anime_staff_positions"]);
        assert_eq!(mapped["manga"], document["published_manga"]);
        assert_eq!(mapped["voices"], document["voice_acting_roles"]);
    }

    #[test]
    fn person_anime_collection_maps_items() {
        let items = vec![json!({
            "position": "Theme Song Performance",
            "anime": {
                "mal_id": 3080,
                "url": "https://myanimelist.net/anime/3080/Anime_Tenchou",
                "images": {
                    "jpg": {
                        "image_url": "https://cdn.myanimelist.net/images/anime/9/4635.jpg",
                        "small_image_url": "https://cdn.myanimelist.net/images/anime/9/4635t.jpg",
                        "large_image_url": "https://cdn.myanimelist.net/images/anime/9/4635l.jpg"
                    },
                    "webp": {
                        "image_url": "https://cdn.myanimelist.net/images/anime/9/4635.webp",
                        "small_image_url": "https://cdn.myanimelist.net/images/anime/9/4635t.webp",
                        "large_image_url": "https://cdn.myanimelist.net/images/anime/9/4635l.webp"
                    }
                },
                "title": "Anime Tenchou"
            }
        })];

        assert_eq!(
            person_anime_collection(&items),
            vec![json!({
                "position": "Theme Song Performance",
                "anime": {
                    "mal_id": 3080,
                    "url": "https://myanimelist.net/anime/3080/Anime_Tenchou",
                    "images": {
                        "jpg": {
                            "image_url": "https://cdn.myanimelist.net/images/anime/9/4635.jpg",
                            "small_image_url": "https://cdn.myanimelist.net/images/anime/9/4635t.jpg",
                            "large_image_url": "https://cdn.myanimelist.net/images/anime/9/4635l.jpg"
                        },
                        "webp": {
                            "image_url": "https://cdn.myanimelist.net/images/anime/9/4635.webp",
                            "small_image_url": "https://cdn.myanimelist.net/images/anime/9/4635t.webp",
                            "large_image_url": "https://cdn.myanimelist.net/images/anime/9/4635l.webp"
                        }
                    },
                    "title": "Anime Tenchou"
                }
            })]
        );
    }

    #[test]
    fn person_manga_collection_maps_items() {
        let items = vec![json!({
            "position": "Art",
            "manga": {
                "mal_id": 3080,
                "url": "https://myanimelist.net/manga/3080/x",
                "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/9/4635.jpg"}},
                "title": "Anime Tenchou"
            }
        })];

        assert_eq!(
            person_manga_collection(&items),
            vec![json!({
                "position": "Art",
                "manga": {
                    "mal_id": 3080,
                    "url": "https://myanimelist.net/manga/3080/x",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/9/4635.jpg"}},
                    "title": "Anime Tenchou"
                }
            })]
        );
    }

    #[test]
    fn person_voices_collection_maps_items() {
        let items = vec![json!({
            "role": "Main",
            "anime": {
                "mal_id": 53127,
                "url": "https://myanimelist.net/anime/53127/Fate_strange_Fake__Whispers_of_Dawn",
                "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/1502/128320.jpg"}},
                "title": "Fate/strange Fake: Whispers of Dawn"
            },
            "character": {
                "mal_id": 2514,
                "url": "https://myanimelist.net/character/2514/Gilgamesh",
                "images": {
                    "jpg": {"image_url": "https://cdn.myanimelist.net/r/84x124/images/characters/12/338672.jpg"},
                    "webp": {
                        "image_url": "https://cdn.myanimelist.net/r/84x124/images/characters/12/338672.webp",
                        "small_image_url": "https://cdn.myanimelist.net/r/84x124/images/characters/12/338672t.webp"
                    }
                },
                "name": "Gilgamesh"
            }
        })];

        assert_eq!(
            person_voices_collection(&items),
            vec![json!({
                "role": "Main",
                "anime": {
                    "mal_id": 53127,
                    "url": "https://myanimelist.net/anime/53127/Fate_strange_Fake__Whispers_of_Dawn",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/1502/128320.jpg"}},
                    "title": "Fate/strange Fake: Whispers of Dawn"
                },
                "character": {
                    "mal_id": 2514,
                    "url": "https://myanimelist.net/character/2514/Gilgamesh",
                    "images": {
                        "jpg": {"image_url": "https://cdn.myanimelist.net/r/84x124/images/characters/12/338672.jpg"},
                        "webp": {
                            "image_url": "https://cdn.myanimelist.net/r/84x124/images/characters/12/338672.webp",
                            "small_image_url": "https://cdn.myanimelist.net/r/84x124/images/characters/12/338672t.webp"
                        }
                    },
                    "name": "Gilgamesh"
                }
            })]
        );
    }

    #[test]
    fn person_full_missing_keys_emit_nulls() {
        let mapped = person_full(&json!({}));
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "website_url",
                "images",
                "name",
                "given_name",
                "family_name",
                "alternate_names",
                "birthday",
                "favorites",
                "about",
                "anime",
                "manga",
                "voices",
            ],
        );
        for key in ["anime", "manga", "voices", "favorites", "birthday"] {
            assert_eq!(mapped[key], Value::Null, "key {key}");
        }
    }

    #[test]
    fn person_pictures_is_the_shared_pictures_resource() {
        let payload = json!({
            "pictures": [
                {"jpg": {"image_url": "https://cdn.myanimelist.net/images/voiceactors/10/34138.jpg"}}
            ]
        });
        assert_eq!(
            pictures(&payload),
            json!([
                {"jpg": {"image_url": "https://cdn.myanimelist.net/images/voiceactors/10/34138.jpg"}}
            ])
        );
        assert_eq!(person_pictures(&payload), pictures(&payload));
    }

    #[test]
    fn person_search_response_builds_pagination_envelope() {
        let pagination = Pagination::search(1, false, 1, 1, 1, 25);
        let mapped = person_search_response(&pagination, &[json!({"mal_id": 7})]);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 1,
                "has_next_page": false,
                "current_page": 1,
                "items": {"count": 1, "total": 1, "per_page": 25}
            })
        );
        assert_eq!(mapped["data"][0]["mal_id"], json!(7));
    }
}
