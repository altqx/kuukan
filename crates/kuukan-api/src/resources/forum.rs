//! Forum resources.
//!
//! `ForumResource.php` returns `$this['topics']` for `/anime/{id}/forum` and
//! `/manga/{id}/forum`; the handlers cache
//! `{"topics": [ForumTopic, ...]}`, and Laravel wraps the bare list as
//! `{"data": [...]}`. [`crate::resources::misc::forum`] shapes each stored
//! topic through [`forum_topic`] so a response always carries the full
//! `ForumTopic` key set, and keeps `null` when the cached document has no
//! `topics` at all.
//!
//! `ForumTopic` JMS shape:
//! `{mal_id, url, title, date, author_username, author_url, comments,
//! last_comment: {url, author_username, author_url, date}}`.

use serde_json::{json, Value};

use crate::resources::misc;

fn get(payload: &Value, key: &str) -> Value {
    misc::get(payload, key)
}

/// `ForumResource::toArray()`: the cached `topics` value (`null` when absent,
/// like `CachedData::__get()`). Callers wrap it with
/// `kuukan_core::envelope::data`.
pub fn forum(payload: &Value) -> Value {
    misc::forum(payload)
}

/// `Jikan\Model\Forum\ForumTopic` JMS shape.
pub fn forum_topic(payload: &Value) -> Value {
    json!({
        "mal_id": get(payload, "mal_id"),
        "url": get(payload, "url"),
        "title": get(payload, "title"),
        "date": get(payload, "date"),
        "author_username": get(payload, "author_username"),
        "author_url": get(payload, "author_url"),
        "comments": get(payload, "comments"),
        "last_comment": match payload.get("last_comment") {
            Some(Value::Object(_)) => forum_post(&payload["last_comment"]),
            other => other.cloned().unwrap_or(Value::Null),
        },
    })
}

/// `Jikan\Model\Forum\ForumPost` JMS shape (`last_comment`).
pub fn forum_post(payload: &Value) -> Value {
    json!({
        "url": get(payload, "url"),
        "author_username": get(payload, "author_username"),
        "author_url": get(payload, "author_url"),
        "date": get(payload, "date"),
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

    fn topic_doc() -> Value {
        json!({
            "mal_id": 2022869,
            "url": "https://myanimelist.net/forum/?topicid=2022869",
            "title": "What was the reception like when this first came out?",
            "date": "2022-06-15T00:00:00+00:00",
            "author_username": "NextUniverse",
            "author_url": "https://myanimelist.net/profile/NextUniverse",
            "comments": 7,
            "last_comment": {
                "url": "https://myanimelist.net/forum/?topicid=2022869&goto=lastpost",
                "author_username": "Bacon_and_Eggs",
                "author_url": "https://myanimelist.net/profile/Bacon_and_Eggs",
                "date": "2022-06-19T06:26:00+00:00"
            }
        })
    }

    #[test]
    fn forum_returns_cached_topics_value() {
        let doc = json!({"topics": [topic_doc()]});
        let topics = forum(&doc);
        assert_eq!(topics.as_array().unwrap().len(), 1);
        assert_eq!(topics[0]["mal_id"], json!(2022869));
        assert_eq!(
            topics[0]["last_comment"]["author_username"],
            json!("Bacon_and_Eggs")
        );
        // PHP array access on a missing key yields null, not [].
        assert!(forum(&json!({})).is_null());
    }

    #[test]
    fn forum_topic_shape() {
        let out = forum_topic(&topic_doc());
        assert_eq!(
            keys(&out),
            vec![
                "author_url",
                "author_username",
                "comments",
                "date",
                "last_comment",
                "mal_id",
                "title",
                "url",
            ]
        );
        assert_eq!(out["comments"], json!(7));
        assert_eq!(
            out["last_comment"]["date"],
            json!("2022-06-19T06:26:00+00:00")
        );
    }

    #[test]
    fn forum_post_shape() {
        let out = forum_post(&topic_doc()["last_comment"]);
        assert_eq!(
            keys(&out),
            vec!["author_url", "author_username", "date", "url"]
        );
        assert_eq!(
            out["url"],
            json!("https://myanimelist.net/forum/?topicid=2022869&goto=lastpost")
        );
        assert!(forum_post(&json!({}))["date"].is_null());
    }
}
