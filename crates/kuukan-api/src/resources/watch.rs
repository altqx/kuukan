//! Watch resources, ported from `WatchController` + the `Query*Episodes` /
//! `Query*PromoVideos` handlers.
//!
//! - `/watch/episodes` and `/watch/episodes/popular`
//!   (`QueryRecentlyAddedEpisodesHandler`, `QueryPopularEpisodesHandler`)
//!   return `Jikan\Model\Watch\Episodes` through the default `ResultsResource`
//!   (`{"pagination": {last_visible_page, has_next_page}, "data": [...]}`).
//!   Items are `Model\Watch\EpisodeListItem` documents:
//!   `{entry, episodes: [RecentEpisodeListItem], region_locked}`.
//! - `/watch/promos` and `/watch/promos/popular`
//!   (`QueryRecentlyAddedPromoVideosHandler`, `QueryPopularPromoVideosHandler`)
//!   return `Model\Watch\PromotionalVideos` through `ResultsResource`; items are
//!   `PromotionalVideoListItem` documents: `{title, entry, trailer}`.
//!
//! The `ResultsResource` port is shared in [`crate::resources::misc`]; the item
//! mappers below document/normalize the model JMS shapes
//! (`StreamEpisodeListItem`/`PromoListItem` are the `/anime/{id}/videos`
//! equivalents, mirrored here as requested).

use serde_json::{json, Value};

use crate::resources::misc;

/// `/watch/episodes` and `/watch/episodes/popular` response body.
pub fn watch_episodes(payload: &Value) -> Value {
    misc::results(payload)
}

/// `/watch/promos` and `/watch/promos/popular` response body.
pub fn watch_promos(payload: &Value) -> Value {
    misc::results(payload)
}

/// `Jikan\Model\Watch\RecentEpisodeListItem` JMS shape:
/// `{mal_id, url, title, premium}`.
pub fn recent_episode_item(payload: &Value) -> Value {
    json!({
        "mal_id": misc::get(payload, "mal_id"),
        "url": misc::get(payload, "url"),
        "title": misc::get(payload, "title"),
        "premium": misc::get(payload, "premium"),
    })
}

/// `Jikan\Model\Watch\EpisodeListItem` JMS shape:
/// `{entry, episodes, region_locked}`.
pub fn watch_episode_item(payload: &Value) -> Value {
    json!({
        "entry": misc::get(payload, "entry"),
        "episodes": misc::get(payload, "episodes"),
        "region_locked": misc::get(payload, "region_locked"),
    })
}

/// `Jikan\Model\Watch\PromotionalVideoListItem` JMS shape:
/// `{title, entry, trailer}`.
pub fn promo_item(payload: &Value) -> Value {
    json!({
        "title": misc::get(payload, "title"),
        "entry": misc::get(payload, "entry"),
        "trailer": misc::get(payload, "trailer"),
    })
}

/// `Jikan\Model\Anime\StreamEpisodeListItem` JMS shape:
/// `{mal_id, title, episode, url, images: {jpg: {image_url}}}`.
pub fn stream_episode_list_item(payload: &Value) -> Value {
    json!({
        "mal_id": misc::get(payload, "mal_id"),
        "title": misc::get(payload, "title"),
        "episode": misc::get(payload, "episode"),
        "url": misc::get(payload, "url"),
        "images": misc::get(payload, "images"),
    })
}

/// `Jikan\Model\Anime\PromoListItem` JMS shape: `{title, trailer}`.
pub fn promo_list_item(payload: &Value) -> Value {
    json!({
        "title": misc::get(payload, "title"),
        "trailer": misc::get(payload, "trailer"),
    })
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

    fn watch_episode_doc() -> Value {
        json!({
            "entry": {
                "mal_id": 21,
                "url": "https://myanimelist.net/anime/21/One_Piece",
                "images": {
                    "jpg": {"image_url": "https://cdn.myanimelist.net/images/anime/6/73245.jpg", "small_image_url": "t", "large_image_url": "l"},
                    "webp": {"image_url": "w", "small_image_url": "tw", "large_image_url": "lw"}
                },
                "title": "One Piece"
            },
            "episodes": [
                {"mal_id": 1022, "url": "https://myanimelist.net/anime/21/One_Piece/episode/1022", "title": "Episode 1022", "premium": false},
                {"mal_id": 1021, "url": "https://myanimelist.net/anime/21/One_Piece/episode/1021", "title": "Episode 1021", "premium": false}
            ],
            "region_locked": false
        })
    }

    #[test]
    fn watch_episodes_matches_results_resource_envelope() {
        let doc = json!({
            "results": [watch_episode_doc()],
            "last_visible_page": 1,
            "has_next_page": false
        });
        let out = watch_episodes(&doc);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 1, "has_next_page": false})
        );
        let item = &out["data"][0];
        assert_eq!(keys(item), vec!["entry", "episodes", "region_locked"]);
        assert_eq!(item["entry"]["mal_id"], json!(21));
        assert_eq!(item["episodes"].as_array().unwrap().len(), 2);
        assert_eq!(item["episodes"][0]["premium"], json!(false));
    }

    #[test]
    fn watch_episodes_honours_stored_pagination() {
        let out = watch_episodes(&json!({
            "results": [],
            "last_visible_page": 5,
            "has_next_page": true
        }));
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 5, "has_next_page": true})
        );
    }

    fn promo_doc() -> Value {
        json!({
            "title": "Character PV",
            "entry": {
                "mal_id": 51019,
                "url": "https://myanimelist.net/anime/51019/Kimetsu_no_Yaiba__Katanakaji_no_Sato-hen",
                "images": {"jpg": {"image_url": "x", "small_image_url": "y", "large_image_url": "z"}, "webp": {"image_url": "x", "small_image_url": "y", "large_image_url": "z"}},
                "title": "Kimetsu no Yaiba: Katanakaji no Sato-hen"
            },
            "trailer": {
                "youtube_id": "t0d7_6WCls8",
                "url": "https://www.youtube.com/watch?v=t0d7_6WCls8",
                "embed_url": "https://www.youtube.com/embed/t0d7_6WCls8?enablejsapi=1&wmode=opaque&autoplay=1",
                "images": {
                    "image_url": "https://img.youtube.com/vi/t0d7_6WCls8/default.jpg",
                    "small_image_url": "https://img.youtube.com/vi/t0d7_6WCls8/sddefault.jpg",
                    "medium_image_url": "https://img.youtube.com/vi/t0d7_6WCls8/mqdefault.jpg",
                    "large_image_url": "https://img.youtube.com/vi/t0d7_6WCls8/hqdefault.jpg",
                    "maximum_image_url": "https://img.youtube.com/vi/t0d7_6WCls8/maxresdefault.jpg"
                }
            }
        })
    }

    #[test]
    fn watch_promos_matches_results_resource_envelope() {
        let out = watch_promos(&json!({"results": [promo_doc()]}));
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 1, "has_next_page": false})
        );
        let item = &out["data"][0];
        assert_eq!(keys(item), vec!["entry", "title", "trailer"]);
        assert_eq!(item["title"], json!("Character PV"));
        assert_eq!(item["trailer"]["youtube_id"], json!("t0d7_6WCls8"));
        assert_eq!(item["trailer"]["images"]["maximum_image_url"], json!("https://img.youtube.com/vi/t0d7_6WCls8/maxresdefault.jpg"));
    }

    #[test]
    fn recent_episode_item_shape() {
        let out = recent_episode_item(&json!({
            "mal_id": 1022,
            "url": "https://myanimelist.net/anime/21/One_Piece/episode/1022",
            "title": "Episode 1022",
            "premium": false
        }));
        assert_eq!(keys(&out), vec!["mal_id", "premium", "title", "url"]);
        assert_eq!(out["mal_id"], json!(1022));
        assert_eq!(out["premium"], json!(false));
        assert_eq!(recent_episode_item(&json!({}))["premium"], json!(null));
    }

    #[test]
    fn watch_episode_item_shape() {
        let out = watch_episode_item(&watch_episode_doc());
        assert_eq!(keys(&out), vec!["entry", "episodes", "region_locked"]);
        assert_eq!(out["region_locked"], json!(false));
    }

    #[test]
    fn promo_item_shape() {
        let out = promo_item(&promo_doc());
        assert_eq!(keys(&out), vec!["entry", "title", "trailer"]);
    }

    #[test]
    fn stream_episode_list_item_shape() {
        let out = stream_episode_list_item(&json!({
            "mal_id": 1,
            "title": "Episode 1",
            "episode": "1",
            "url": "https://myanimelist.net/anime/1/episode/1",
            "images": {"jpg": {"image_url": "x"}}
        }));
        assert_eq!(keys(&out), vec!["episode", "images", "mal_id", "title", "url"]);
        assert_eq!(out["episode"], json!("1"));
        assert_eq!(out["images"]["jpg"]["image_url"], json!("x"));
    }

    #[test]
    fn promo_list_item_shape() {
        let out = promo_list_item(&json!({"title": "PV", "trailer": {"youtube_id": "abc"}}));
        assert_eq!(keys(&out), vec!["title", "trailer"]);
        assert_eq!(out["trailer"]["youtube_id"], json!("abc"));
    }
}
