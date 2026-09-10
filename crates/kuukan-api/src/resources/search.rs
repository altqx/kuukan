//! Search/list collections.
//!
//! Ported 1:1 from the `ResourceCollection` classes under
//! `app/Http/Resources/V4/`:
//!
//! - [`anime_collection`] / [`anime_search`] — `AnimeCollection`
//! - [`manga_collection`] / [`manga_search`] — `MangaCollection`
//! - [`character_collection`] / [`character_search`] — `CharacterCollection`
//! - [`person_collection`] / [`person_search`] — `PersonCollection`
//! - [`club_collection`] / [`club_search`] — `ClubCollection`
//! - [`user_collection`] / [`user_search`] — `UserCollection`
//! - [`producer_collection`] / [`producer_search`] — `ProducerCollection`
//! - [`magazine_collection`] / [`magazine_search`] — `MagazineCollection`
//! - [`genre_collection`] / [`genre_search`] — `GenreCollection`
//!
//! In PHP each collection's `$collects` class is applied to every underlying
//! item before `toArray()` builds the envelope. The search item shapes are
//! therefore mapped by [`anime`](super::anime::anime),
//! [`manga`](super::manga::manga) and the local single-item mappers below
//! (which mirror the sibling `*Resource` classes until those land).
//!
//! Envelope notes:
//! - `AnimeCollection`/`MangaCollection`/`CharacterCollection`/`PersonCollection`/
//!   `ProducerCollection`/`MagazineCollection` emit the full `pagination_plus`
//!   block (`last_visible_page`, `has_next_page`, `current_page`, `items`).
//! - `ClubCollection` emits only `last_visible_page` + `has_next_page`
//!   ([`Pagination::list`]).
//! - `UserCollection` and `GenreCollection` emit `{data: [...]}` with no
//!   pagination block.

use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::{json, Value};

use crate::resources::misc::{get, get_first};

// ---------------------------------------------------------------------------
// Anime / manga (item mappers live in the sibling modules).
// ---------------------------------------------------------------------------

/// `AnimeCollection`: map each stored item through `AnimeResource`.
pub fn anime_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(super::anime::anime).collect()
}

/// `AnimeCollection::toArray`: pagination_plus + mapped items.
pub fn anime_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, anime_collection(items))
}

/// `MangaCollection`: map each stored item through `MangaResource`.
pub fn manga_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(super::manga::manga).collect()
}

/// `MangaCollection::toArray`: pagination_plus + mapped items.
pub fn manga_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, manga_collection(items))
}

// ---------------------------------------------------------------------------
// Search item resources (single item shapes used by the collections).
// ---------------------------------------------------------------------------

/// `CharacterResource` item shape.
pub fn character_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "name": get(payload, "name"),
        "name_kanji": get(payload, "name_kanji"),
        "nicknames": get(payload, "nicknames"),
        // PHP reads the `favorites` accessor, which is backed by `member_favorites`.
        "favorites": get_first(payload, &["favorites", "member_favorites"]),
        "about": get(payload, "about"),
    })
}

/// `CharacterCollection`: map each stored item through `CharacterResource`.
pub fn character_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(character_item).collect()
}

/// `CharacterCollection::toArray`: pagination_plus + mapped items.
pub fn character_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, character_collection(items))
}

/// `PersonResource` item shape.
pub fn person_item(payload: &Value) -> Value {
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
        // PHP reads the `favorites` accessor, which is backed by `member_favorites`.
        "favorites": get_first(payload, &["favorites", "member_favorites"]),
        "about": get(payload, "about"),
    })
}

/// `PersonCollection`: map each stored item through `PersonResource`.
pub fn person_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(person_item).collect()
}

/// `PersonCollection::toArray`: pagination_plus + mapped items.
pub fn person_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, person_collection(items))
}

/// `ClubResource` item shape.
pub fn club_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "images": get(payload, "images"),
        "name": get(payload, "name"),
        "members": get(payload, "members"),
        "category": get(payload, "category"),
        "created": get(payload, "created"),
        "access": get(payload, "access"),
    })
}

/// `ClubCollection`: map each stored item through `ClubResource`.
pub fn club_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(club_item).collect()
}

/// `ClubCollection::toArray`: plain pagination + mapped items.
pub fn club_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, club_collection(items))
}

/// `ProfileResource` item shape (`UserCollection::$collects`).
pub fn user_item(payload: &Value) -> Value {
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

/// `UserCollection`: map each stored item through `ProfileResource`.
pub fn user_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(user_item).collect()
}

/// `UserCollection::toArray`: `{data: [...]}` (no pagination block).
pub fn user_search(items: &[Value]) -> Value {
    envelope::data(user_collection(items))
}

/// `ProducerResource` item shape.
pub fn producer_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "titles": get(payload, "titles"),
        "images": get(payload, "images"),
        "favorites": get(payload, "favorites"),
        "established": get(payload, "established"),
        "about": get(payload, "about"),
        "count": get(payload, "count"),
    })
}

/// `ProducerCollection`: map each stored item through `ProducerResource`.
pub fn producer_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(producer_item).collect()
}

/// `ProducerCollection::toArray`: pagination_plus + mapped items.
pub fn producer_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, producer_collection(items))
}

/// `MagazineResource` item shape.
pub fn magazine_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "name": get(payload, "name"),
        "url": get(payload, "url"),
        "count": get(payload, "count"),
    })
}

/// `MagazineCollection`: map each stored item through `MagazineResource`.
pub fn magazine_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(magazine_item).collect()
}

/// `MagazineCollection::toArray`: pagination_plus + mapped items.
pub fn magazine_search(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, magazine_collection(items))
}

/// `GenreResource` item shape.
pub fn genre_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "name": get(payload, "name"),
        "url": get(payload, "url"),
        "count": get(payload, "count"),
    })
}

/// `GenreCollection`: map each stored item through `GenreResource`.
pub fn genre_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(genre_item).collect()
}

/// `GenreCollection::toArray`: `{data: [...]}` (no pagination block).
pub fn genre_search(items: &[Value]) -> Value {
    envelope::data(genre_collection(items))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search_pagination() -> Pagination {
        Pagination::search(2, true, 2, 5, 30, 25)
    }

    #[test]
    fn anime_collection_maps_items_through_anime_resource() {
        let items = vec![json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/anime/1/Cowboy_Bebop",
            "title": "Cowboy Bebop",
            "type": "TV",
        })];

        let mapped = anime_collection(&items);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0]["mal_id"], json!(1));
        assert_eq!(mapped[0]["title"], json!("Cowboy Bebop"));
        // AnimeResource defaults
        assert_eq!(mapped[0]["approved"], json!(true));
        assert_eq!(mapped[0]["titles"], json!([]));
        assert_eq!(mapped[0].as_object().unwrap().len(), 36);
    }

    #[test]
    fn anime_search_builds_pagination_plus_envelope() {
        // tests/Integration/AnimeSearchEndpointTest + AnimeCollection
        let items = vec![json!({})];
        let mapped = anime_search(&search_pagination(), &items);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 2,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 5, "total": 30, "per_page": 25},
            })
        );
        assert_eq!(mapped["data"].as_array().unwrap().len(), 1);
        assert_eq!(mapped["data"][0]["approved"], json!(true));
        assert_eq!(mapped["data"][0]["titles"], json!([]));
        assert_eq!(mapped["data"][0].as_object().unwrap().len(), 36);
        assert_eq!(mapped.as_object().unwrap().len(), 2);
    }

    #[test]
    fn manga_search_builds_pagination_plus_envelope() {
        // tests/Integration/SearchControllerTest::testMangaSearch
        let items = vec![json!({
            "mal_id": 1,
            "title": "Berserk",
            "score": 9.47,
        })];
        let mapped = manga_search(&search_pagination(), &items);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 2,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 5, "total": 30, "per_page": 25},
            })
        );
        // `scored` mirrors `score`
        assert_eq!(mapped["data"][0]["score"], json!(9.47));
        assert_eq!(mapped["data"][0]["scored"], json!(9.47));
        assert_eq!(mapped["data"][0].as_object().unwrap().len(), 30);
    }

    #[test]
    fn character_item_matches_character_resource() {
        // tests/Integration/SearchControllerTest::testCharacterSearch
        let payload = json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/character/1/Spike_Spiegel",
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/characters/9/69275.jpg"},
                "webp": {"image_url": "https://cdn.myanimelist.net/images/characters/9/69275.webp"}
            },
            "name": "Spiegel, Spike",
            "name_kanji": "スパイク・スピーゲル",
            "nicknames": ["Spike"],
            "favorites": 12345,
            "about": "A bounty hunter.",
        });

        assert_eq!(
            character_item(&payload),
            json!({
                "mal_id": 1,
                "url": "https://myanimelist.net/character/1/Spike_Spiegel",
                "images": {
                    "jpg": {"image_url": "https://cdn.myanimelist.net/images/characters/9/69275.jpg"},
                    "webp": {"image_url": "https://cdn.myanimelist.net/images/characters/9/69275.webp"}
                },
                "name": "Spiegel, Spike",
                "name_kanji": "スパイク・スピーゲル",
                "nicknames": ["Spike"],
                "favorites": 12345,
                "about": "A bounty hunter.",
            })
        );

        // JMS/store shape: favorites is backed by member_favorites
        let raw = json!({"mal_id": 1, "name": "x", "member_favorites": 7});
        assert_eq!(character_item(&raw)["favorites"], json!(7));

        // missing keys are null, no defaults
        let mapped = character_item(&json!({}));
        assert_eq!(mapped.as_object().unwrap().len(), 8);
        assert_eq!(mapped["favorites"], Value::Null);
    }

    #[test]
    fn character_search_builds_pagination_plus_envelope() {
        let items = vec![json!({"mal_id": 1, "name": "Okabe"})];
        let mapped = character_search(&search_pagination(), &items);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 2,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 5, "total": 30, "per_page": 25},
            })
        );
        assert_eq!(mapped["data"][0]["name"], json!("Okabe"));
        // CharacterResource emits `name_kanji` even when unresolved
        assert_eq!(mapped["data"][0]["name_kanji"], Value::Null);
    }

    #[test]
    fn person_item_matches_person_resource() {
        // database/factories/PersonFactory.php
        let payload = json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/people/1/Tomokazu_Seki",
            "website_url": "https://website.example",
            "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/voiceactors/1/55486.jpg"}},
            "name": "Seki, Tomokazu",
            "given_name": "Tomokazu",
            "family_name": "Seki",
            "alternate_names": ["Sekki"],
            "birthday": "1972-09-08T00:00:00+00:00",
            "favorites": 500,
            "about": "test",
        });

        let mapped = person_item(&payload);
        assert_eq!(mapped["mal_id"], json!(1));
        assert_eq!(mapped["website_url"], json!("https://website.example"));
        assert_eq!(mapped["alternate_names"], json!(["Sekki"]));
        assert_eq!(mapped["birthday"], json!("1972-09-08T00:00:00+00:00"));
        assert_eq!(mapped["favorites"], json!(500));
        assert_eq!(mapped.as_object().unwrap().len(), 11);

        assert_eq!(
            person_item(&json!({"mal_id": 1, "member_favorites": 9}))["favorites"],
            json!(9)
        );
    }

    #[test]
    fn person_search_builds_pagination_plus_envelope() {
        let items = vec![json!({"mal_id": 1, "name": "Sawano Kuma"})];
        let mapped = person_search(&search_pagination(), &items);

        assert_eq!(mapped["pagination"]["current_page"], json!(2));
        assert_eq!(mapped["pagination"]["items"]["total"], json!(30));
        assert_eq!(mapped["data"][0]["name"], json!("Sawano Kuma"));
    }

    #[test]
    fn club_item_matches_club_resource() {
        // database/factories/ClubFactory.php
        let payload = json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/clubs.php?cid=1",
            "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/clubs/16/222057.jpg"}},
            "category": "anime",
            "created": "2020-01-01T00:00:00+00:00",
            "name": "Cowboy Bebop Club",
            "members": 1234,
            "access": "public",
        });

        assert_eq!(
            club_item(&payload),
            json!({
                "mal_id": 1,
                "url": "https://myanimelist.net/clubs.php?cid=1",
                "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/clubs/16/222057.jpg"}},
                "name": "Cowboy Bebop Club",
                "members": 1234,
                "category": "anime",
                "created": "2020-01-01T00:00:00+00:00",
                "access": "public",
            })
        );
    }

    #[test]
    fn club_search_uses_plain_pagination() {
        let items = vec![json!({"mal_id": 1, "name": "club"})];
        let mapped = club_search(&Pagination::list(3, true), &items);

        assert_eq!(
            mapped,
            json!({
                "pagination": {"last_visible_page": 3, "has_next_page": true},
                "data": [{
                    "mal_id": 1,
                    "url": null,
                    "images": null,
                    "name": "club",
                    "members": null,
                    "category": null,
                    "created": null,
                    "access": null,
                }],
            })
        );
        // no current_page/items for ClubCollection
        assert!(mapped["pagination"].get("current_page").is_none());
        assert!(mapped["pagination"].get("items").is_none());
    }

    #[test]
    fn user_item_matches_profile_resource() {
        // database/factories/ProfileFactory.php (head fields)
        let payload = json!({
            "mal_id": 1,
            "username": "purplepinapples",
            "url": "https://myanimelist.net/profile/purplepinapples",
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/userimages/1.jpg"},
                "webp": {"image_url": "https://cdn.myanimelist.net/images/userimages/1.webp"}
            },
            "last_online": "2023-01-31T22:34:00+00:00",
            "gender": "Male",
            "birthday": "1990-01-01T00:00:00+00:00",
            "location": "Japan",
            "joined": "2010-01-01T00:00:00+00:00",
        });

        assert_eq!(
            user_item(&payload),
            json!({
                "mal_id": 1,
                "username": "purplepinapples",
                "url": "https://myanimelist.net/profile/purplepinapples",
                "images": {
                    "jpg": {"image_url": "https://cdn.myanimelist.net/images/userimages/1.jpg"},
                    "webp": {"image_url": "https://cdn.myanimelist.net/images/userimages/1.webp"}
                },
                "last_online": "2023-01-31T22:34:00+00:00",
                "gender": "Male",
                "birthday": "1990-01-01T00:00:00+00:00",
                "location": "Japan",
                "joined": "2010-01-01T00:00:00+00:00",
            })
        );
    }

    #[test]
    fn user_search_has_data_only() {
        let items = vec![json!({"mal_id": 1, "username": "x"})];
        let mapped = user_search(&items);

        assert_eq!(mapped["data"][0]["username"], json!("x"));
        assert_eq!(mapped.as_object().unwrap().len(), 1);
        assert!(mapped.get("pagination").is_none());
    }

    #[test]
    fn producer_item_and_search() {
        // database/factories/ProducersFactory.php
        let payload = json!({
            "mal_id": 1,
            "url": "https://myanimelist.net/anime/producer/1/Studio_Pierrot",
            "titles": [{"type": "Default", "title": "Studio Pierrot"}],
            "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/company/441.png"}},
            "favorites": 535,
            "established": "1979-05-01T00:00:00+00:00",
            "about": "",
            "count": 42,
        });

        assert_eq!(
            producer_item(&payload),
            json!({
                "mal_id": 1,
                "url": "https://myanimelist.net/anime/producer/1/Studio_Pierrot",
                "titles": [{"type": "Default", "title": "Studio Pierrot"}],
                "images": {"jpg": {"image_url": "https://cdn.myanimelist.net/images/company/441.png"}},
                "favorites": 535,
                "established": "1979-05-01T00:00:00+00:00",
                "about": "",
                "count": 42,
            })
        );

        // ProducerResource `?? null` defaults resolve to null here
        let mapped = producer_item(&json!({}));
        assert_eq!(mapped.as_object().unwrap().len(), 8);
        assert_eq!(mapped["favorites"], Value::Null);
        assert_eq!(mapped["established"], Value::Null);

        let mapped = producer_search(&search_pagination(), &[payload]);
        assert_eq!(mapped["pagination"]["items"]["count"], json!(5));
        assert_eq!(mapped["data"][0]["count"], json!(42));
    }

    #[test]
    fn magazine_item_and_search() {
        // database/factories/MagazineFactory.php
        let payload = json!({
            "mal_id": 1,
            "name": "Big Comic Spirits",
            "url": "https://myanimelist.net/manga/magazine/1/Big_Comic_Spirits",
            "count": 129,
        });

        assert_eq!(
            magazine_item(&payload),
            json!({
                "mal_id": 1,
                "name": "Big Comic Spirits",
                "url": "https://myanimelist.net/manga/magazine/1/Big_Comic_Spirits",
                "count": 129,
            })
        );

        let mapped = magazine_search(&search_pagination(), &[payload]);
        assert_eq!(mapped["pagination"]["last_visible_page"], json!(2));
        assert_eq!(mapped["data"][0]["name"], json!("Big Comic Spirits"));
    }

    #[test]
    fn genre_item_and_search() {
        // tests/Integration/GenreControllerTest
        let payload = json!({
            "mal_id": 1,
            "name": "Action",
            "url": "https://myanimelist.net/anime/genre/1/Action",
            "count": 4000,
        });

        assert_eq!(
            genre_item(&payload),
            json!({
                "mal_id": 1,
                "name": "Action",
                "url": "https://myanimelist.net/anime/genre/1/Action",
                "count": 4000,
            })
        );

        let mapped = genre_search(&[payload]);
        assert_eq!(mapped.as_object().unwrap().len(), 1);
        assert_eq!(mapped["data"][0]["name"], json!("Action"));
        assert!(mapped.get("pagination").is_none());
    }

    #[test]
    fn collections_handle_empty_input() {
        assert!(anime_collection(&[]).is_empty());
        assert!(character_collection(&[]).is_empty());
        assert_eq!(user_search(&[]), json!({"data": []}));
        assert_eq!(genre_collection(&[]), Vec::<Value>::new());
    }
}
