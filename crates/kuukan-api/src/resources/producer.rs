//! Producer response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/ProducerResource.php`,
//! `ProducerFullResource.php`, `ExternalLinksResource.php` (used by the
//! `/producers/{id}/external` handler) and `ProducerCollection.php` (search).
//!
//! Stored JMS documents carry the producer `malUrl` under the serialized name
//! `meta` (`storage/app/metadata.v4/Jikan.Model.Producer.Producer.yml`); the
//! resources never read it, so it is intentionally dropped from the output.

use crate::resources::misc::{self, get};
use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::{json, Value};

/// `ProducerResource::toArray()`.
///
/// `favorites`, `established`, `about` and `count` are explicitly coalesced
/// with `null` in PHP; the other fields rely on `CachedData::__get()` which
/// already returns `null` for absent keys.
pub fn producer(payload: &Value) -> Value {
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

/// `ProducerFullResource::toArray()` — `producer` plus `external`.
pub fn producer_full(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "titles": get(payload, "titles"),
        "images": get(payload, "images"),
        "favorites": get(payload, "favorites"),
        "established": get(payload, "established"),
        "about": get(payload, "about"),
        "count": get(payload, "count"),
        "external": get(payload, "external_links"),
    })
}

/// `ExternalLinksResource::toArray()` — returns the raw `external_links`
/// value. Also used by the anime/manga/user external endpoints; delegates to
/// the canonical [`crate::resources::misc::external_links`] mapper.
pub fn external_links(payload: &Value) -> Value {
    misc::external_links(payload)
}

/// `ProducerCollection` item mapping (search results; `$collects` is
/// `ProducerResource`).
pub fn producer_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(producer).collect()
}

/// Envelope for `ProducerCollection`: `{"pagination": {...}, "data": [...]}`.
pub fn producer_search_response(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, producer_collection(items))
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

    /// Shaped after `ProducersFactory` (includes the `meta` serialized
    /// `malUrl` so tests prove it is not emitted).
    fn producer_document() -> Value {
        json!({
            "mal_id": 441,
            "url": "https://myanimelist.net/anime/producer/441/Eiken",
            "meta": {
                "mal_id": 441,
                "type": "anime/producer",
                "name": "Eiken",
                "url": "https://myanimelist.net/anime/producer/441/Eiken"
            },
            "images": {
                "jpg": {"image_url": "https://cdn.myanimelist.net/images/company/441.png"}
            },
            "name": "Eiken",
            "titles": [{"type": "Default", "title": "Eiken"}],
            "favorites": 535,
            "established": "2011-06-14T00:00:00+00:00",
            "about": "",
            "external_links": [
                {"name": "Official Site", "url": "https://www.eiken-anime.jp/"}
            ],
            "count": 42
        })
    }

    #[test]
    fn producer_maps_all_fields_and_drops_meta() {
        let mapped = producer(&producer_document());
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "titles",
                "images",
                "favorites",
                "established",
                "about",
                "count",
            ],
        );
        assert_eq!(mapped["mal_id"], json!(441));
        assert_eq!(
            mapped["url"],
            json!("https://myanimelist.net/anime/producer/441/Eiken")
        );
        assert_eq!(
            mapped["titles"],
            json!([{"type": "Default", "title": "Eiken"}])
        );
        assert_eq!(mapped["favorites"], json!(535));
        assert_eq!(mapped["established"], json!("2011-06-14T00:00:00+00:00"));
        assert_eq!(mapped["about"], json!(""));
        assert_eq!(mapped["count"], json!(42));
        assert!(mapped.get("meta").is_none());
    }

    #[test]
    fn producer_coalesces_missing_fields_to_null() {
        assert_eq!(
            producer(&json!({"mal_id": 1, "url": "https://myanimelist.net/anime/producer/1"})),
            json!({
                "mal_id": 1,
                "url": "https://myanimelist.net/anime/producer/1",
                "titles": null,
                "images": null,
                "favorites": null,
                "established": null,
                "about": null,
                "count": null,
            })
        );
    }

    #[test]
    fn producer_full_adds_external() {
        let mapped = producer_full(&producer_document());
        assert_object_keys(
            &mapped,
            &[
                "mal_id",
                "url",
                "titles",
                "images",
                "favorites",
                "established",
                "about",
                "count",
                "external",
            ],
        );
        assert_eq!(
            mapped["external"],
            json!([{"name": "Official Site", "url": "https://www.eiken-anime.jp/"}])
        );
    }

    #[test]
    fn producer_full_external_defaults_to_null() {
        assert_eq!(producer_full(&json!({}))["external"], Value::Null);
    }

    #[test]
    fn external_links_returns_raw_value() {
        let payload = json!({
            "external_links": [
                {"name": "Official Site", "url": "https://www.eiken-anime.jp/"}
            ]
        });
        assert_eq!(
            external_links(&payload),
            json!([{"name": "Official Site", "url": "https://www.eiken-anime.jp/"}])
        );
        assert_eq!(external_links(&json!({})), Value::Null);
    }

    #[test]
    fn producer_search_response_builds_pagination_envelope() {
        let pagination = Pagination::search(3, true, 2, 30, 60, 30);
        let mapped = producer_search_response(&pagination, &[producer_document()]);

        assert_eq!(
            mapped["pagination"],
            json!({
                "last_visible_page": 3,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 30, "total": 60, "per_page": 30}
            })
        );
        assert_object_keys(
            &mapped["data"][0],
            &[
                "mal_id",
                "url",
                "titles",
                "images",
                "favorites",
                "established",
                "about",
                "count",
            ],
        );
    }
}
