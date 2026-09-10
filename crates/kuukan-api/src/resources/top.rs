//! Top resources, ported from `app/Http/Controllers/V4DB/TopController.php`
//! and the handlers behind it.
//!
//! - `/top/anime` (`QueryTopAnimeItemsHandler`) → `AnimeCollection`
//! - `/top/manga` (`QueryTopMangaItemsHandler`) → `MangaCollection`
//! - `/top/characters` (`QueryTopCharactersHandler`) → `CharacterCollection`
//! - `/top/people` (`QueryTopPeopleHandler`) → `PersonCollection`
//! - `/top/reviews` (`QueryTopReviewsHandler`) → default `ResultsResource`
//!
//! All four item collections are search-style (`pagination_plus` + `data`);
//! `/top/reviews` is list-style. Item mapping is the canonical
//! [`crate::resources::search`] one (`AnimeResource`, `MangaResource`,
//! `CharacterResource`, `PersonResource`); review items pass through.

use kuukan_core::envelope;
use kuukan_core::pagination::Pagination;
use serde_json::Value;

use crate::resources::{misc, search};

/// `AnimeCollection` envelope for `/top/anime`.
pub fn top_anime(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::anime_collection(items))
}

/// `MangaCollection` envelope for `/top/manga`.
pub fn top_manga(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::manga_collection(items))
}

/// `CharacterCollection` envelope for `/top/characters`.
pub fn top_characters(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::character_collection(items))
}

/// `PersonCollection` envelope for `/top/people`.
pub fn top_people(pagination: &Pagination, items: &[Value]) -> Value {
    envelope::paged(pagination, search::person_collection(items))
}

/// `/top/reviews` (default `ResultsResource`).
pub fn top_reviews(payload: &Value) -> Value {
    misc::results(payload)
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
    fn top_anime_matches_anime_search_shape() {
        let items = vec![
            json!({"mal_id": 5114, "title": "Fullmetal Alchemist: Brotherhood", "rank": 1, "score": 9.1}),
            json!({"mal_id": 9253, "title": "Steins;Gate", "rank": 2, "score": 9.07}),
        ];
        let pagination = Pagination::search(2, true, 1, 2, 30, 25);
        let out = top_anime(&pagination, &items);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(out["data"][0]["title"], json!("Fullmetal Alchemist: Brotherhood"));
        // canonical AnimeResource defaults are applied
        assert_eq!(out["data"][0]["approved"], json!(true));
        assert_eq!(
            out["pagination"],
            json!({
                "last_visible_page": 2,
                "has_next_page": true,
                "current_page": 1,
                "items": {"count": 2, "total": 30, "per_page": 25}
            })
        );
    }

    #[test]
    fn top_manga_characters_people_use_their_item_resources() {
        let pagination = Pagination::search(1, false, 1, 1, 1, 25);
        let manga = top_manga(&pagination, &[json!({"mal_id": 1, "title": "Berserk"})]);
        assert_eq!(manga["data"][0]["scored"], manga["data"][0]["score"]);
        let character = top_characters(&pagination, &[json!({"mal_id": 1, "name": "Okabe"})]);
        assert!(character["data"][0].get("name_kanji").is_some());
        assert!(character["data"][0].get("favorites").is_some());
        let person = top_people(
            &pagination,
            &[json!({"mal_id": 1, "name": "Seki, Tomokazu", "member_favorites": 500})],
        );
        assert_eq!(person["data"][0]["favorites"], json!(500));
        assert!(person["data"][0].get("website_url").is_some());
    }

    #[test]
    fn top_reviews_uses_results_resource_envelope() {
        let doc = json!({
            "results": [
                {
                    "mal_id": 448579,
                    "url": "https://myanimelist.net/reviews.php?id=448579",
                    "type": "anime",
                    "reactions": {"overall": 0, "nice": 0, "love_it": 0, "funny": 0, "confusing": 0, "informative": 0, "well_written": 0, "creative": 0},
                    "date": "2022-06-20T12:13:00+00:00",
                    "review": "Its good",
                    "score": 4,
                    "tags": ["Recommended"],
                    "is_spoiler": false,
                    "is_preliminary": false,
                    "episodes_watched": 12,
                    "entry": {"mal_id": 43470, "url": "https://myanimelist.net/anime/43470", "images": {"jpg": {"image_url": "x"}}, "title": "Rikei"},
                    "user": {"url": "https://myanimelist.net/profile/helmy47", "username": "helmy47", "images": {"jpg": {"image_url": "i"}}}
                }
            ],
            "last_visible_page": 2,
            "has_next_page": true
        });
        let out = top_reviews(&doc);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 2, "has_next_page": true})
        );
        assert_eq!(out["data"][0]["episodes_watched"], json!(12));
        assert_eq!(out["data"][0]["entry"]["mal_id"], json!(43470));
        // ResultsResource defaults when pagination is absent
        let empty = top_reviews(&json!({}));
        assert_eq!(
            empty["pagination"],
            json!({"last_visible_page": 1, "has_next_page": false})
        );
        assert!(empty["data"].is_null());
    }
}
