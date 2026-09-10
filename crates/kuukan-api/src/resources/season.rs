//! Season resources.
//!
//! There are no dedicated PHP resource classes for seasons: the endpoints are
//! built from shared collections in the handlers.
//!
//! - `/seasons` (`SeasonController@archive`) → `QueryAnimeSeasonListHandler`
//!   returns `Jikan\Model\SeasonList\SeasonArchive` through the default
//!   `ResultsResource` (`{"pagination": {last_visible_page, has_next_page}, "data": [...]}`).
//!   Items are `SeasonListItem` documents: `{year, seasons}`.
//! - `/seasons/now` (`SeasonController@now`) → `QueryCurrentAnimeSeasonHandler`
//!   returns `AnimeCollection` (search-style `pagination_plus` envelope).
//! - `/seasons/{year}/{season}` (`SeasonController@main`) →
//!   `QuerySpecificAnimeSeasonHandler` returns `AnimeCollection`.
//! - `/seasons/upcoming` (`SeasonController@later`) →
//!   `QueryUpcomingAnimeSeasonHandler` returns `AnimeCollection`.
//!
//! The anime item mapping is the canonical [`crate::resources::search`] one.

use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::Value;

use crate::resources::{misc, search};

/// `ResultsResource` envelope for `/seasons`.
pub fn season_archive(payload: &Value) -> Value {
    misc::results(payload)
}

/// `AnimeCollection` envelope for `/seasons/now`.
pub fn season_now(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::anime_collection(items))
}

/// `AnimeCollection` envelope for `/seasons/{year}/{season}`.
pub fn season_main(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::anime_collection(items))
}

/// `AnimeCollection` envelope for `/seasons/upcoming`.
pub fn season_upcoming(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::anime_collection(items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }

    fn search_pagination() -> Pagination {
        Pagination::search(3, true, 2, 2, 42, 2)
    }

    #[test]
    fn season_archive_uses_results_resource_envelope() {
        let doc = json!({
            "results": [
                {"year": 2020, "seasons": ["winter", "spring", "summer", "fall"]},
                {"year": 2019, "seasons": ["winter", "spring", "summer", "fall"]}
            ],
            "last_visible_page": 1,
            "has_next_page": false
        });
        let out = season_archive(&doc);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 1, "has_next_page": false})
        );
        assert_eq!(
            out["data"][0],
            json!({"year": 2020, "seasons": ["winter", "spring", "summer", "fall"]})
        );
    }

    #[test]
    fn season_archive_honours_stored_pagination() {
        let doc = json!({
            "results": [],
            "last_visible_page": 5,
            "has_next_page": true
        });
        let out = season_archive(&doc);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 5, "has_next_page": true})
        );
    }

    #[test]
    fn season_now_builds_search_pagination_and_maps_items() {
        let items = vec![json!({
            "mal_id": 54857,
            "title": "Re:Zero kara Hajimeru Isekai Seikatsu 3rd Season",
            "type": "TV",
            "aired": {"from": "2024-10-02T00:00:00+00:00"}
        })];
        let out = season_now(&search_pagination(), &items);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({
                "last_visible_page": 3,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 2, "total": 42, "per_page": 2}
            })
        );
        assert_eq!(out["data"][0]["mal_id"], json!(54857));
        assert_eq!(out["data"][0]["type"], json!("TV"));
        // canonical AnimeResource defaults are applied
        assert_eq!(out["data"][0]["approved"], json!(true));
        assert_eq!(out["data"][0]["titles"], json!([]));
    }

    #[test]
    fn upcoming_and_specific_share_the_collection_envelope() {
        let items = vec![json!({"mal_id": 9})];
        assert_eq!(
            season_upcoming(&search_pagination(), &items),
            season_now(&search_pagination(), &items)
        );
        assert_eq!(
            season_main(&search_pagination(), &items),
            season_now(&search_pagination(), &items)
        );
    }

    #[test]
    fn season_collections_handle_empty_items() {
        let out = season_now(&Pagination::search(1, false, 1, 0, 0, 25), &[]);
        assert_eq!(out["data"], json!([]));
    }
}
