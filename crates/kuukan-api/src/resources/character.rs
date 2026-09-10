//! Character response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/CharacterResource.php`,
//! `CharacterFullResource.php`, `CharacterAnimeResource.php` (+
//! `CharacterAnimeCollection`), `CharacterMangaResource.php` (+
//! `CharacterMangaCollection`), `CharacterSeiyuuResource.php` (+
//! `CharacterSeiyuuCollection`), `CharacterCollection.php` (search) and
//! `PicturesResource.php` (character pictures).
//!
//! Mappers take the stored/cached document (`serde_json::Value`, JMS
//! snake_case shape) and reproduce the PHP resource's `toArray($request)`
//! array exactly. Absent keys become JSON `null`, like `CachedData::__get()`
//! returning `null` for a missing collection key; PHP emits those nulls
//! because `setSerializeNull(true)` is set.

use crate::resources::misc::{self, get, get_first};
use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::{json, Value};

/// `CharacterResource` reads `$this->favorites`, a model accessor over the
/// stored `member_favorites` column. JMS scraper documents carry
/// `member_favorites`; model-shaped documents (Eloquent `toArray`) carry
/// `favorites`. Accept either, preferring the explicit accessor key.
fn favorites(payload: &Value) -> Value {
    get_first(payload, &["favorites", "member_favorites"])
}

/// `CharacterResource::toArray()`.
///
/// ```text
/// {mal_id, url, images, name, name_kanji, nicknames, favorites, about}
/// ```
pub fn character(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "name": get(payload, "name"),
        "name_kanji": get(payload, "name_kanji"),
        "nicknames": get(payload, "nicknames"),
        "favorites": favorites(payload),
        "about": get(payload, "about"),
    })
}

/// `CharacterFullResource::toArray()`.
///
/// `anime`, `manga` and `voices` are the raw `animeography`, `mangaography`
/// and `voice_actors` lists (the PHP resource does not map them through the
/// per-item resources).
pub fn character_full(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "name": get(payload, "name"),
        "name_kanji": get(payload, "name_kanji"),
        "nicknames": get(payload, "nicknames"),
        "favorites": favorites(payload),
        "about": get(payload, "about"),
        "anime": get(payload, "animeography"),
        "manga": get(payload, "mangaography"),
        "voices": get(payload, "voice_actors"),
    })
}

/// `CharacterAnimeResource::toArray()` for one `animeography` item.
pub fn character_anime(item: &Value) -> Value {
    let anime = item.get("anime").cloned().unwrap_or(Value::Null);
    json!({
        "role": get(item, "role"),
        "anime": {
            "mal_id": get(&anime, "mal_id"),
            "url": get(&anime, "url"),
            "images": get(&anime, "images"),
            "title": get(&anime, "title"),
        },
    })
}

/// `CharacterAnimeCollection` item mapping.
pub fn character_anime_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(character_anime).collect()
}

/// `CharacterMangaResource::toArray()` for one `mangaography` item.
pub fn character_manga(item: &Value) -> Value {
    let manga = item.get("manga").cloned().unwrap_or(Value::Null);
    json!({
        "role": get(item, "role"),
        "manga": {
            "mal_id": get(&manga, "mal_id"),
            "url": get(&manga, "url"),
            "images": get(&manga, "images"),
            "title": get(&manga, "title"),
        },
    })
}

/// `CharacterMangaCollection` item mapping.
pub fn character_manga_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(character_manga).collect()
}

/// `CharacterSeiyuuResource::toArray()` for one `voice_actors` item.
pub fn character_seiyuu(item: &Value) -> Value {
    let person = item.get("person").cloned().unwrap_or(Value::Null);
    json!({
        "language": get(item, "language"),
        "person": {
            "mal_id": get(&person, "mal_id"),
            "url": get(&person, "url"),
            "images": get(&person, "images"),
            "name": get(&person, "name"),
        },
    })
}

/// `CharacterSeiyuuCollection` item mapping.
pub fn character_seiyuu_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(character_seiyuu).collect()
}

/// `PicturesResource::toArray()` — returns the raw `pictures` value.
///
/// Shared with `/people/{id}/pictures` (and the anime/manga picture
/// endpoints); delegates to the canonical [`crate::resources::misc::pictures`]
/// mapper.
pub fn pictures(payload: &Value) -> Value {
    misc::pictures(payload)
}

/// Alias of [`pictures`] for `/characters/{id}/pictures`.
pub fn character_pictures(payload: &Value) -> Value {
    pictures(payload)
}

/// `CharacterCollection` item mapping (search results; `$collects` is
/// `CharacterResource`).
pub fn character_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(character).collect()
}

/// Envelope for `CharacterCollection`: `{"pagination": {...}, "data": [...]}`.
pub fn character_search_response(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, character_collection(items))
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
                "image_url": "https://cdn.myanimelist.net/images/characters/4/50197.jpg"
            },
            "webp": {
                "image_url": "https://cdn.myanimelist.net/images/characters/4/50197.webp",
                "small_image_url": "https://cdn.myanimelist.net/images/characters/4/50197t.webp"
            }
        })
    }

    fn jms_character() -> Value {
        json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/character/1/Spike_Spiegel",
            "images": images(),
            "name": "Spiegel, Spike",
            "name_kanji": "スパイク・スピーゲル",
            "nicknames": ["Spike"],
            "member_favorites": 34604,
            "about": "Spike Spiegel is a bounty hunter.",
            "animeography": [],
            "mangaography": [],
            "voice_actors": []
        })
    }

    #[test]
    fn character_maps_all_fields() {
        let mapped = character(&jms_character());
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "images",
                "name",
                "name_kanji",
                "nicknames",
                "favorites",
                "about",
            ],
        );
        assert_eq!(mapped["mal_id"], json!(1));
        assert_eq!(
            mapped["url"],
            json!("https://myanimelist.net/character/1/Spike_Spiegel")
        );
        assert_eq!(mapped["images"], images());
        assert_eq!(mapped["name"], json!("Spiegel, Spike"));
        assert_eq!(mapped["name_kanji"], json!("スパイク・スピーゲル"));
        assert_eq!(mapped["nicknames"], json!(["Spike"]));
        assert_eq!(mapped["favorites"], json!(34604));
        assert_eq!(mapped["about"], json!("Spike Spiegel is a bounty hunter."));
    }

    #[test]
    fn character_reads_model_shaped_favorites() {
        let mut document = jms_character();
        let map = document.as_object_mut().unwrap();
        map.remove("member_favorites");
        map.insert("favorites".to_string(), json!(42));
        assert_eq!(character(&document)["favorites"], json!(42));
    }

    #[test]
    fn character_emits_nulls_for_missing_keys() {
        assert_eq!(
            character(&json!({})),
            json!({
                "mal_id": null,
                "url": null,
                "images": null,
                "name": null,
                "name_kanji": null,
                "nicknames": null,
                "favorites": null,
                "about": null,
            })
        );
    }

    #[test]
    fn character_full_inlines_ographies() {
        let document = json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/character/1",
            "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/characters/4/50197.jpg"}},
            "name": "Spiegel, Spike",
            "name_kanji": null,
            "nicknames": [],
            "member_favorites": 7,
            "about": null,
            "animeography": [{
                "role": "Main",
                "anime": {
                    "mal_id": 1,
                    "url": "https://myanimelist.net/anime/1/Cowboy_Bebop",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/4/19644.jpg"}},
                    "title": "Cowboy Bebop"
                }
            }],
            "mangaography": [{
                "role": "Main",
                "manga": {
                    "mal_id": 1,
                    "url": "https://myanimelist.net/manga/1/Cowboy_Bebop",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/manga/4/19644.jpg"}},
                    "title": "Cowboy Bebop"
                }
            }],
            "voice_actors": [{
                "language": "Japanese",
                "person": {
                    "mal_id": 1,
                    "url": "https://myanimelist.net/people/1",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/voiceactors/1/1.jpg"}},
                    "name": "Yamadera, Kouichi"
                }
            }]
        });

        let mapped = character_full(&document);
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "images",
                "name",
                "name_kanji",
                "nicknames",
                "favorites",
                "about",
                "anime",
                "manga",
                "voices",
            ],
        );
        // The full resource emits the raw lists, not the per-item mappings.
        assert_eq!(mapped["anime"], document["animeography"]);
        assert_eq!(mapped["manga"], document["mangaography"]);
        assert_eq!(mapped["voices"], document["voice_actors"]);
        assert_eq!(mapped["favorites"], json!(7));
    }

    #[test]
    fn character_anime_collection_maps_items() {
        let items = vec![
            json!({
                "role": "Main",
                "anime": {
                    "mal_id": 1,
                    "url": "https://myanimelist.net/anime/1/Cowboy_Bebop",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/4/19644.jpg"}},
                    "title": "Cowboy Bebop"
                }
            }),
            json!({
                "role": "Supporting",
                "anime": {
                    "mal_id": 6,
                    "url": "https://myanimelist.net/anime/6/Trigun",
                    "images": null,
                    "title": "Trigun"
                }
            }),
        ];

        let mapped = character_anime_collection(&items);
        assert_eq!(
            mapped,
            vec![
                json!({
                    "role": "Main",
                    "anime": {
                        "mal_id": 1,
                        "url": "https://myanimelist.net/anime/1/Cowboy_Bebop",
                        "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/4/19644.jpg"}},
                        "title": "Cowboy Bebop"
                    }
                }),
                json!({
                    "role": "Supporting",
                    "anime": {
                        "mal_id": 6,
                        "url": "https://myanimelist.net/anime/6/Trigun",
                        "images": null,
                        "title": "Trigun"
                    }
                }),
            ]
        );
    }

    #[test]
    fn character_anime_item_without_anime_keeps_object_with_nulls() {
        let mapped = character_anime(&json!({"role": "Main"}));
        assert_eq!(
            mapped,
            json!({
                "role": "Main",
                "anime": {"mal_id": null, "url": null, "images": null, "title": null}
            })
        );
    }

    #[test]
    fn character_manga_collection_maps_items() {
        let items = vec![json!({
            "role": "Main",
            "manga": {
                "mal_id": 13,
                "url": "https://myanimelist.net/manga/13/Fullmetal_Alchemist",
                "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/manga/3/243675.jpg"}},
                "title": "Fullmetal Alchemist"
            }
        })];

        assert_eq!(
            character_manga_collection(&items),
            vec![json!({
                "role": "Main",
                "manga": {
                    "mal_id": 13,
                    "url": "https://myanimelist.net/manga/13/Fullmetal_Alchemist",
                    "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/manga/3/243675.jpg"}},
                    "title": "Fullmetal Alchemist"
                }
            })]
        );
    }

    #[test]
    fn character_seiyuu_collection_maps_items() {
        let items = vec![json!({
            "language": "Japanese",
            "person": {
                "mal_id": 357,
                "url": "https://myanimelist.net/people/357/Unshou_Ishizuk",
                "images": {
                    "jpg": {
                        "image_url": "https://cdn.myanimelist.net/images/voiceactors/2/17135.jpg?s=5925123b8a7cf9b51a445c225442f0ef"
                    }
                },
                "name": "Ishizuka, Unshou"
            }
        })];

        assert_eq!(
            character_seiyuu_collection(&items),
            vec![json!({
                "language": "Japanese",
                "person": {
                    "mal_id": 357,
                    "url": "https://myanimelist.net/people/357/Unshou_Ishizuk",
                    "images": {
                        "jpg": {
                            "image_url": "https://cdn.myanimelist.net/images/voiceactors/2/17135.jpg?s=5925123b8a7cf9b51a445c225442f0ef"
                        }
                    },
                    "name": "Ishizuka, Unshou"
                }
            })]
        );
    }

    #[test]
    fn character_pictures_returns_the_pictures_value() {
        let payload = json!({
            "pictures": [
                {"jpg": {"image_url": "https://cdn.myanimelist.net/images/characters/10/34138.jpg"}}
            ]
        });
        assert_eq!(
            pictures(&payload),
            json!([
                {"jpg": {"image_url": "https://cdn.myanimelist.net/images/characters/10/34138.jpg"}}
            ])
        );
        assert_eq!(pictures(&json!({})), Value::Null);
        assert_eq!(character_pictures(&payload), pictures(&payload));
    }

    #[test]
    fn character_collection_maps_each_item_through_character() {
        let items = vec![
            json!({"mal_id": 1, "favorites": 10, "nicknames": []}),
            json!({"mal_id": 2, "member_favorites": 20, "nicknames": ["A"]}),
        ];
        let mapped = character_collection(&items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0]["favorites"], json!(10));
        assert_eq!(mapped[1]["favorites"], json!(20));
        assert_object_keys(
            &mapped[0],
            &[
                "mal_id",
                "url",
                "images",
                "name",
                "name_kanji",
                "nicknames",
                "favorites",
                "about",
            ],
        );
    }

    #[test]
    fn character_search_response_builds_pagination_envelope() {
        let pagination = Pagination::search(2, true, 1, 25, 30, 25);
        let mapped = character_search_response(&pagination, &[json!({"mal_id": 1})]);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 2,
                "has_next_page": true,
                "current_page": 1,
                "items": {"count": 25, "total": 30, "per_page": 25}
            })
        );
        assert_eq!(mapped["data"][0]["mal_id"], json!(1));
        assert_object_keys(
            &mapped["data"][0],
            &[
                "mal_id",
                "url",
                "images",
                "name",
                "name_kanji",
                "nicknames",
                "favorites",
                "about",
            ],
        );
    }
}
