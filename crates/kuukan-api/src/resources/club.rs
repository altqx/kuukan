//! Club response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/ClubResource.php`,
//! `ClubStaffResource.php`, `ClubRelationsResource.php`,
//! `ClubCollection.php` (search) and the club-members path
//! (`ClubMembersLookupHandler` + `ResultsResource.php`).
//!
//! `ClubCollection` only emits `last_visible_page` / `has_next_page` (no
//! `current_page` / `items` block), unlike the `pagination_plus` search
//! collections. `ClubMembersLookupHandler` uses the default
//! `RequestHandlerWithScraperCache` resource, i.e. `ResultsResource`, which
//! also only emits those two pagination keys and defaults them to `1` /
//! `false`.

use crate::resources::misc::{self, get};
use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::{json, Value};

/// `ClubResource::toArray()`.
///
/// ```text
/// {mal_id, url, images, name, members, category, created, access}
/// ```
pub fn club(payload: &Value) -> Value {
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

/// `ClubStaffResource::toArray()` — returns the raw `staff` value.
pub fn club_staff(payload: &Value) -> Value {
    get(payload, "staff")
}

/// `ClubRelationsResource::toArray()`.
pub fn club_relations(payload: &Value) -> Value {
    json!({
        "anime": get(payload, "anime"),
        "manga": get(payload, "manga"),
        "characters": get(payload, "characters"),
    })
}

/// `ClubCollection` item mapping (`$collects` is `ClubResource`).
pub fn club_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(club).collect()
}

/// Envelope for `ClubCollection`: `{"pagination": {last_visible_page,
/// has_next_page}, "data": [...]}`.
pub fn club_search_response(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, club_collection(items))
}

/// `ResultsResource::toArray()` for `GET /clubs/{id}/members`.
///
/// Unlike the paginator-backed collections, this reads `last_visible_page`,
/// `has_next_page` and `results` straight from the stored document
/// (`ResultsResource` in `RequestHandlerWithScraperCache`); delegates to the
/// canonical [`crate::resources::misc::results`] mapper.
pub fn club_members(payload: &Value) -> Value {
    misc::results(payload)
}

/// Alias of [`club_members`].
pub fn club_members_response(payload: &Value) -> Value {
    club_members(payload)
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

    /// Shaped after `ClubFactory`.
    fn club_document() -> Value {
        json!({
            "mal_id": 222057,
            "url": "https://myanimelist.net/clubs.php?cid=222057",
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/clubs/16/222057.jpg"}
            },
            "category": "anime",
            "created": "2008-10-03T00:00:00+00:00",
            "created_at": "2008-10-03T00:00:00+00:00",
            "updated_at": "2008-10-04T00:00:00+00:00",
            "name": "Cowboy Bebop Club",
            "request_hash": "request:v4:abc",
            "anime": [
                {"mal_id": 1, "type": "anime", "name": "Cowboy Bebop", "url": "https://myanimelist.net/anime/1/x"}
            ],
            "characters": [
                {"mal_id": 1, "type": "character", "name": "Spike Spiegel", "url": "https://myanimelist.net/character/1"}
            ],
            "manga": [
                {"mal_id": 1, "type": "manga", "name": "Cowboy Bebop", "url": "https://myanimelist.net/manga/1/x"}
            ],
            "staff": [
                {"url": "https://myanimelist.net/profile/cyruz", "username": "cryuz"}
            ],
            "members": 1234,
            "access": "public"
        })
    }

    #[test]
    fn club_maps_all_fields() {
        let mapped = club(&club_document());
        assert_object_keys(
            &mapped,
            &[
                "mal_id", "url", "images", "name", "members", "category", "created", "access",
            ],
        );
        assert_eq!(mapped["mal_id"], json!(222057));
        assert_eq!(
            mapped["url"],
            json!("https://myanimelist.net/clubs.php?cid=222057")
        );
        assert_eq!(
            mapped["images"],
            json!({"jpg": {"image_url": "https://cdn.myanimelist.net/images/clubs/16/222057.jpg"}})
        );
        assert_eq!(mapped["name"], json!("Cowboy Bebop Club"));
        assert_eq!(mapped["members"], json!(1234));
        assert_eq!(mapped["category"], json!("anime"));
        assert_eq!(mapped["created"], json!("2008-10-03T00:00:00+00:00"));
        assert_eq!(mapped["access"], json!("public"));
        assert!(mapped.get("anime").is_none());
    }

    #[test]
    fn club_emits_nulls_for_missing_keys() {
        assert_eq!(
            club(&json!({})),
            json!({
                "mal_id": null,
                "url": null,
                "images": null,
                "name": null,
                "members": null,
                "category": null,
                "created": null,
                "access": null,
            })
        );
    }

    #[test]
    fn club_staff_returns_raw_staff() {
        let payload = json!({
            "staff": [{"url": "https://myanimelist.net/profile/cyruz", "username": "cryuz"}]
        });
        assert_eq!(
            club_staff(&payload),
            json!([{"url": "https://myanimelist.net/profile/cyruz", "username": "cryuz"}])
        );
        assert_eq!(club_staff(&json!({})), Value::Null);
    }

    #[test]
    fn club_relations_maps_three_sections() {
        let document = club_document();
        assert_eq!(
            club_relations(&document),
            json!({
                "anime": document["anime"],
                "manga": document["manga"],
                "characters": document["characters"],
            })
        );
        assert_eq!(
            club_relations(&json!({})),
            json!({"anime": null, "manga": null, "characters": null})
        );
    }

    #[test]
    fn club_collection_maps_items() {
        let items = vec![club_document(), json!({"mal_id": 2, "name": "Naruto Club"})];
        let mapped = club_collection(&items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[1]["name"], json!("Naruto Club"));
        assert_object_keys(
            &mapped[0],
            &[
                "mal_id", "url", "images", "name", "members", "category", "created", "access",
            ],
        );
    }

    #[test]
    fn club_search_response_only_has_two_pagination_keys() {
        let pagination = Pagination::list(5, true);
        let mapped = club_search_response(&pagination, &[club_document()]);

        assert_object_keys(&mapped, &["pagination", "data"]);
        assert_eq!(
            mapped["pagination"],
            json!({"last_visible_page": 5, "has_next_page": true})
        );
        assert_eq!(mapped["data"][0]["mal_id"], json!(222057));
    }

    #[test]
    fn club_members_builds_results_envelope() {
        let document = json!({
            "results": [{
                "username": "cyruz",
                "url": "https://myanimelist.net/profile/cyruz",
                "images": {
                    "jpg": {"image_url": "http://httpbin.org/get"},
                    "webp": {"image_url": "http://httpbin.org/get"}
                }
            }],
            "has_next_page": true,
            "last_visible_page": 7,
            "createdAt": "2024-01-02T00:00:00+00:00",
            "modifiedAt": "2024-01-02T00:00:00+00:00",
            "request_hash": "request:clubs:abc"
        });

        let mapped = club_members(&document);
        assert_object_keys(&mapped, &["pagination", "data"]);
        assert_eq!(
            mapped["pagination"],
            json!({"last_visible_page": 7, "has_next_page": true})
        );
        assert_eq!(mapped["data"], document["results"]);
    }

    #[test]
    fn club_members_defaults_pagination_when_missing() {
        let mapped = club_members(&json!({"results": []}));
        assert_eq!(
            mapped,
            json!({
                "pagination": {"last_visible_page": 1, "has_next_page": false},
                "data": []
            })
        );
        assert_eq!(club_members_response(&json!({"results": []})), mapped);
    }
}
