//! Schedule resources.
//!
//! `/schedules` (`ScheduleController@main`) is built in
//! `QueryAnimeSchedulesHandler` (no dedicated PHP resource class): the
//! repository query is paginated with Laravel's `LengthAwarePaginator` and
//! wrapped in `AnimeCollection`, so the response is the `anime_search` shape
//! (`pagination_plus` + `data`). The anime item mapping is the canonical
//! [`crate::resources::search`] one.

use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::Value;

use crate::resources::search;

/// `AnimeCollection` envelope for `/schedules`.
pub fn schedules(pagination: &Pagination, items: &[Value]) -> Value {
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

    #[test]
    fn schedules_matches_anime_collection_envelope() {
        let items = vec![
            json!({"mal_id": 1, "title": "Cowboy Bebop", "type": "TV", "status": "Currently Airing", "airing": true, "members": 100}),
            json!({"mal_id": 2, "title": "Trigun", "type": "TV", "status": "Currently Airing", "airing": true, "members": 50}),
            json!({"mal_id": 3, "title": "Berserk", "type": "TV", "status": "Currently Airing", "airing": true, "members": 10}),
        ];
        let pagination = Pagination::search(1, false, 1, 3, 3, 25);
        let out = schedules(&pagination, &items);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(out["pagination"]["current_page"], json!(1));
        assert_eq!(out["pagination"]["last_visible_page"], json!(1));
        assert_eq!(out["pagination"]["has_next_page"], json!(false));
        assert_eq!(
            out["pagination"]["items"],
            json!({"count": 3, "total": 3, "per_page": 25})
        );
        assert_eq!(out["data"].as_array().unwrap().len(), 3);
        assert_eq!(out["data"][0]["members"], json!(100));
        // canonical AnimeResource defaults are applied to each item
        assert_eq!(out["data"][0]["approved"], json!(true));
        assert_eq!(out["data"][0]["titles"], json!([]));
    }

    #[test]
    fn schedules_keeps_pagination_plus_values() {
        let pagination = Pagination::search(4, true, 2, 1, 7, 1);
        let out = schedules(&pagination, &[json!({"mal_id": 1})]);
        assert_eq!(
            out["pagination"],
            json!({
                "last_visible_page": 4,
                "has_next_page": true,
                "current_page": 2,
                "items": {"count": 1, "total": 7, "per_page": 1}
            })
        );
    }

    #[test]
    fn schedules_empty_items() {
        let out = schedules(&Pagination::list(1, false), &[]);
        assert_eq!(
            out,
            json!({
                "pagination": {
                    "last_visible_page": 1,
                    "has_next_page": false
                },
                "data": []
            })
        );
    }
}
