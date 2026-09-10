//! Genre response resources.
//!
//! Ported 1:1 from `app/Http/Resources/V4/GenreResource.php` and
//! `GenreCollection.php`, used by `GenreController@anime` /
//! `GenreController@manga` (`GET /genres/anime`, `GET /genres/manga`) through
//! `GenreListHandler`.
//!
//! The handler feeds rows from the genre tables (`genres_anime`,
//! `explicit_genres_anime`, `themes_anime`, `demographics_anime` and the
//! manga equivalents); all of them share the
//! `{mal_id, name, url, count}` shape. `GenreCollection::toArray()` returns
//! the bare collection, which Laravel wraps as `{"data": [...]}` (list
//! endpoints without pagination).
//!
//! Stored JMS documents for `Jikan\Model\Genre\AnimeGenre` /
//! `MangaGenre` carry the genre `malUrl` under the serialized name `meta`
//! (`storage/app/metadata.v4/Jikan.Model.Genre.*.yml`); `GenreResource` never
//! reads it.

use crate::resources::misc::get;
use kuukan_core::envelope;
use serde_json::{json, Value};

/// `GenreResource::toArray()`.
///
/// ```text
/// {mal_id, name, url, count}
/// ```
pub fn genre(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "name": get(payload, "name"),
        "url": get(payload, "url"),
        "count": get(payload, "count"),
    })
}

/// `GenreCollection` item mapping (`$collects` is `GenreResource`).
pub fn genre_collection(items: &[Value]) -> Vec<Value> {
    items.iter().map(genre).collect()
}

/// Root-level response for `/genres/anime` and `/genres/manga`:
/// `{"data": [...]}` (no pagination block).
pub fn genre_list_response(items: &[Value]) -> Value {
    envelope::data(genre_collection(items))
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

    /// Shaped after a `genres_anime` row plus the `meta` serialized `malUrl`
    /// of the scraper document, which `GenreResource` ignores.
    fn genre_document() -> Value {
        json!({
            "mal_id": 1,
            "name": "Action",
            "url": "https://myanimelist.net/anime/genre/1/Action",
            "count": 5442,
            "meta": {
                "mal_id": 1,
                "type": "anime/genre",
                "name": "Action",
                "url": "https://myanimelist.net/anime/genre/1/Action"
            }
        })
    }

    #[test]
    fn genre_maps_all_fields_and_drops_meta() {
        let mapped = genre(&genre_document());
        assert_object_keys(&mapped, &["mal_id", "name", "url", "count"]);
        assert_eq!(mapped["mal_id"], json!(1));
        assert_eq!(mapped["name"], json!("Action"));
        assert_eq!(
            mapped["url"],
            json!("https://myanimelist.net/anime/genre/1/Action")
        );
        assert_eq!(mapped["count"], json!(5442));
        assert!(mapped.get("meta").is_none());
    }

    #[test]
    fn genre_emits_nulls_for_missing_keys() {
        assert_eq!(
            genre(&json!({})),
            json!({"mal_id": null, "name": null, "url": null, "count": null})
        );
    }

    #[test]
    fn genre_collection_maps_items() {
        let items = vec![
            genre_document(),
            json!({"mal_id": 62, "name": "Isekai", "url": "https://myanimelist.net/anime/genre/62/Isekai", "count": 0}),
        ];
        let mapped = genre_collection(&items);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[1]["name"], json!("Isekai"));
        assert_eq!(mapped[1]["count"], json!(0));
        assert_object_keys(&mapped[0], &["mal_id", "name", "url", "count"]);
    }

    #[test]
    fn genre_list_response_wraps_in_data() {
        let mapped = genre_list_response(&[genre_document()]);
        assert_object_keys(&mapped, &["data"]);
        assert_eq!(mapped["data"][0]["mal_id"], json!(1));
        assert_eq!(genre_list_response(&[])["data"], json!([]));
    }
}
