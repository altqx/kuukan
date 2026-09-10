//! Magazine response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/MagazineResource.php` and
//! `MagazineCollection.php` (used by `MagazineSearchHandler` for
//! `GET /magazines`).
//!
//! `MagazineCollection` builds a `pagination_plus` block itself and strips
//! Laravel's `links`/`meta` keys; the Rust envelope is built with
//! `kuukan_core::envelope::paged` + `kuukan_core::pagination::Pagination`.
//!
//! Stored JMS documents carry the magazine `malUrl` under the serialized name
//! `meta` (`Jikan.Model.Magazine.Magazine.yml`); `MagazineResource` never
//! reads it.

use crate::resources::misc::get;
use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::{json, Value};

/// `MagazineResource::toArray()`.
///
/// ```text
/// {mal_id, name, url, count}
/// ```
pub fn magazine(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "name": get(payload, "name"),
        "url": get(payload, "url"),
        "count": get(payload, "count"),
    })
}

/// `MagazineCollection` item mapping (`$collects` is `MagazineResource`).
pub fn magazine_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(magazine).collect()
}

/// Envelope for `MagazineCollection`: `{"pagination": {...}, "data": [...]}`.
pub fn magazine_search_response(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, magazine_collection(items))
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

    /// Shaped after `MagazineFactory` (includes `meta`, which the resource
    /// ignores).
    fn magazine_document() -> Value {
        json!({
            "mal_id": 1,
            "name": "Big Comic Spirits",
            "url": "https://myanimelist.net/manga/magazine/1/Big_Comic_Spirits",
            "meta": {
                "mal_id": 1,
                "type": "manga/magazine",
                "name": "Big Comic Spirits",
                "url": "https://myanimelist.net/manga/magazine/1/Big_Comic_Spirits"
            },
            "count": 288
        })
    }

    #[test]
    fn magazine_maps_all_fields_and_drops_meta() {
        let mapped = magazine(&magazine_document());
        assert_object_keys(&mapped, &["mal_id", "name", "url", "count"]);
        assert_eq!(mapped["mal_id"], json!(1));
        assert_eq!(mapped["name"], json!("Big Comic Spirits"));
        assert_eq!(
            mapped["url"],
            json!("https://myanimelist.net/manga/magazine/1/Big_Comic_Spirits")
        );
        assert_eq!(mapped["count"], json!(288));
        assert!(mapped.get("meta").is_none());
    }

    #[test]
    fn magazine_emits_nulls_for_missing_keys() {
        assert_eq!(
            magazine(&json!({})),
            json!({"mal_id": null, "name": null, "url": null, "count": null})
        );
    }

    #[test]
    fn magazine_collection_maps_items() {
        let items = vec![
            magazine_document(),
            json!({"mal_id": 2, "name": "Young Jump", "url": "https://myanimelist.net/manga/magazine/2/Young_Jump", "count": 0}),
        ];
        let mapped = magazine_collection(&items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[1]["name"], json!("Young Jump"));
        assert_eq!(mapped[1]["count"], json!(0));
        assert_object_keys(&mapped[0], &["mal_id", "name", "url", "count"]);
    }

    #[test]
    fn magazine_search_response_builds_pagination_envelope() {
        let pagination = Pagination::search(4, true, 2, 25, 76, 25);
        let mapped = magazine_search_response(&pagination, &[magazine_document()]);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 4,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 25, "total": 76, "per_page": 25}
            })
        );
        assert_eq!(mapped["data"][0]["mal_id"], json!(1));
    }
}
