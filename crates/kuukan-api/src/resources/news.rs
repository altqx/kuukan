//! News resources.
//!
//! `NewsResource.php` wraps the `Model\News\NewsList` payload
//! (`{"results": [...], "has_next_page": bool, "last_visible_page": int}`) as
//! `{"pagination": {last_visible_page ?? 1, has_next_page ?? false}, "data": [...]}`.
//! The shared port lives in [`crate::resources::misc::news`].
//!
//! In jikan-rest v4.2.2 the anime/manga news handlers use the default
//! `ResultsResource`, which has the same shape. The `NewsListItem` JMS shape is
//! `{mal_id, url, title, date, author_username, author_url, forum_url,
//! images: {jpg: {image_url}}, comments, excerpt}`.

use serde_json::{json, Value};

use crate::resources::misc;

fn get(payload: &Value, key: &str) -> Value {
    misc::get(payload, key)
}

/// `NewsResource::toArray()`.
pub fn news(payload: &Value) -> Value {
    misc::news(payload)
}

/// `Jikan\Model\News\NewsListItem` JMS shape.
pub fn news_item(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "title": get(payload, "title"),
        "date": get(payload, "date"),
        "author_username": get(payload, "author_username"),
        "author_url": get(payload, "author_url"),
        "forum_url": get(payload, "forum_url"),
        "images": get(payload, "images"),
        "comments": get(payload, "comments"),
        "excerpt": get(payload, "excerpt"),
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

    fn news_item_doc() -> Value {
        json!({
            "mal_id": 60609964,
            "url": "https://myanimelist.net/news/60609964",
            "title": "North American Anime & Manga Releases for September",
            "date": "2020-08-31T14:34:00+00:00",
            "author_username": "ImperfectBlue",
            "author_url": "https://myanimelist.net/profile/ImperfectBlue",
            "forum_url": "https://myanimelist.net/forum/?topicid=1862079",
            "images": {
                "jpg": {
                    "image_url": "https://cdn.myanimelist.net/s/common/uploaded_files/1598909553-a6f9acc1b6c36cd7b792e5bd67321c13.png"
                }
            },
            "comments": 0,
            "excerpt": "Here are the North American anime & manga releases for September Week 1"
        })
    }

    #[test]
    fn news_matches_news_resource_envelope() {
        let doc = json!({
            "results": [news_item_doc()],
            "last_visible_page": 4,
            "has_next_page": true
        });
        let out = news(&doc);
        assert_eq!(keys(&out), vec!["data", "pagination"]);
        assert_eq!(
            out["pagination"],
            json!({"last_visible_page": 4, "has_next_page": true})
        );
        let item = &out["data"][0];
        assert_eq!(
            keys(item),
            vec![
                "author_url",
                "author_username",
                "comments",
                "date",
                "excerpt",
                "forum_url",
                "images",
                "mal_id",
                "title",
                "url",
            ]
        );
        assert_eq!(item["comments"], json!(0));
        assert_eq!(item["images"]["jpg"]["image_url"], json!("https://cdn.myanimelist.net/s/common/uploaded_files/1598909553-a6f9acc1b6c36cd7b792e5bd67321c13.png"));
    }

    #[test]
    fn news_defaults_when_pagination_missing() {
        let out = news(&json!({"results": []}));
        assert_eq!(
            out,
            json!({
                "pagination": {"last_visible_page": 1, "has_next_page": false},
                "data": []
            })
        );
    }

    #[test]
    fn news_item_shape() {
        let out = news_item(&news_item_doc());
        assert_eq!(out["mal_id"], json!(60609964));
        assert_eq!(out["author_username"], json!("ImperfectBlue"));
        let empty = news_item(&json!({}));
        for key in [
            "mal_id",
            "url",
            "title",
            "date",
            "author_username",
            "author_url",
            "forum_url",
            "images",
            "comments",
            "excerpt",
        ] {
            assert!(empty[key].is_null(), "{key} should be null");
        }
    }
}
