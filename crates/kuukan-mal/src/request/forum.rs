//! Forum request paths shared by the anime/manga APIs.
//!
//! Ports of `Jikan\Request\Anime\AnimeForumRequest` and
//! `Jikan\Request\Manga\MangaForumRequest` (the shared parser lives in
//! [`crate::parser::forum`], see [`crate::parser::forum::parse_forum`]).

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// Topic filters accepted by `AnimeForumRequest`/`MangaForumRequest`.
pub const VALID_TOPIC_TYPES: [&str; 3] = ["all", "episode", "other"];

fn forum_query(topic: Option<&str>) -> String {
    match topic {
        Some(topic) if VALID_TOPIC_TYPES.contains(&topic) => {
            format!(
                "?{}",
                http_build_query(&[("topic", QueryValue::Str(topic))])
            )
        }
        _ => String::new(),
    }
}

/// `Jikan\Request\Anime\AnimeForumRequest`.
#[derive(Debug, Clone)]
pub struct AnimeForumRequest {
    id: i64,
    topic: Option<String>,
}

impl AnimeForumRequest {
    /// `new AnimeForumRequest($id, $topic = null)`.
    pub fn new(id: i64, topic: Option<&str>) -> Self {
        AnimeForumRequest {
            id,
            topic: topic.map(|t| t.to_string()),
        }
    }

    /// `getId()`.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// `getTopic()`.
    pub fn topic(&self) -> Option<&str> {
        self.topic.as_deref()
    }
}

impl MalRequest for AnimeForumRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/anime/{}/_/forum{}",
            self.id,
            forum_query(self.topic.as_deref())
        )
    }
}

/// `Jikan\Request\Manga\MangaForumRequest`.
#[derive(Debug, Clone)]
pub struct MangaForumRequest {
    id: i64,
    topic: Option<String>,
}

impl MangaForumRequest {
    /// `new MangaForumRequest($id, $topic = null)`.
    pub fn new(id: i64, topic: Option<&str>) -> Self {
        MangaForumRequest {
            id,
            topic: topic.map(|t| t.to_string()),
        }
    }

    /// `getId()`.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// `getTopic()`.
    pub fn topic(&self) -> Option<&str> {
        self.topic.as_deref()
    }
}

impl MalRequest for MangaForumRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/manga/{}/_/forum{}",
            self.id,
            forum_query(self.topic.as_deref())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php() {
        assert_eq!(
            AnimeForumRequest::new(1, None).path(),
            "https://myanimelist.net/anime/1/_/forum"
        );
        assert_eq!(
            AnimeForumRequest::new(1, Some("episode")).path(),
            "https://myanimelist.net/anime/1/_/forum?topic=episode"
        );
        assert_eq!(
            AnimeForumRequest::new(1, Some("bogus")).path(),
            "https://myanimelist.net/anime/1/_/forum"
        );
        assert_eq!(
            MangaForumRequest::new(2, Some("all")).path(),
            "https://myanimelist.net/manga/2/_/forum?topic=all"
        );
    }
}
