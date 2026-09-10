//! Port of `Jikan\Request\Manga\*` (jikan-php v4.0.12).
//!
//! Every `path()` reproduces the PHP `getPath()` byte for byte, including the
//! odd placeholders (`/_/` slugs, `jikan` path segment) and query strings.

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// Topics accepted by [`MangaForumRequest`] (`MangaForumRequest::$validTypes`).
pub const MANGA_FORUM_VALID_TYPES: [&str; 3] = ["all", "episode", "other"];

/// `Jikan\Request\Manga\MangaRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaRequest {
    pub id: i64,
}

impl MangaRequest {
    pub fn new(id: i64) -> Self {
        MangaRequest { id }
    }
}

impl MalRequest for MangaRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/{}/", self.id)
    }
}

/// `Jikan\Request\Manga\MangaCharactersRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaCharactersRequest {
    pub id: i64,
}

impl MangaCharactersRequest {
    pub fn new(id: i64) -> Self {
        MangaCharactersRequest { id }
    }
}

impl MalRequest for MangaCharactersRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/{}/_/characters", self.id)
    }
}

/// `Jikan\Request\Manga\MangaPicturesRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaPicturesRequest {
    pub id: i64,
}

impl MangaPicturesRequest {
    pub fn new(id: i64) -> Self {
        MangaPicturesRequest { id }
    }
}

impl MalRequest for MangaPicturesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/{}/jikan/pics", self.id)
    }
}

/// `Jikan\Request\Manga\MangaStatsRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaStatsRequest {
    pub id: i64,
}

impl MangaStatsRequest {
    pub fn new(id: i64) -> Self {
        MangaStatsRequest { id }
    }
}

impl MalRequest for MangaStatsRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/{}/jikan/stats", self.id)
    }
}

/// `Jikan\Request\Manga\MangaMoreInfoRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaMoreInfoRequest {
    pub id: i64,
}

impl MangaMoreInfoRequest {
    pub fn new(id: i64) -> Self {
        MangaMoreInfoRequest { id }
    }
}

impl MalRequest for MangaMoreInfoRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/{}/_/moreinfo", self.id)
    }
}

/// `Jikan\Request\Manga\MangaNewsRequest`.
///
/// PHP: `?p=%d` where the page defaults to 1; an explicit `null` formats as 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaNewsRequest {
    pub id: i64,
    pub page: Option<u64>,
}

impl MangaNewsRequest {
    pub fn new(id: i64) -> Self {
        MangaNewsRequest { id, page: Some(1) }
    }

    pub fn with_page(id: i64, page: Option<u64>) -> Self {
        MangaNewsRequest { id, page }
    }
}

impl MalRequest for MangaNewsRequest {
    fn path(&self) -> String {
        let page = self.page.unwrap_or(0);
        format!("{BASE_URL}/manga/{}/_/news?p={page}", self.id)
    }
}

/// `Jikan\Request\Manga\MangaForumRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaForumRequest {
    pub id: i64,
    pub topic: Option<String>,
}

impl MangaForumRequest {
    pub fn new(id: i64) -> Self {
        MangaForumRequest { id, topic: None }
    }

    pub fn with_topic(id: i64, topic: Option<String>) -> Self {
        MangaForumRequest { id, topic }
    }
}

impl MalRequest for MangaForumRequest {
    fn path(&self) -> String {
        let query = match self.topic.as_deref() {
            Some(topic) if MANGA_FORUM_VALID_TYPES.contains(&topic) => {
                format!(
                    "?{}",
                    http_build_query(&[("topic", QueryValue::Str(topic))])
                )
            }
            _ => String::new(),
        };
        format!("{BASE_URL}/manga/{}/_/forum{query}", self.id)
    }
}

/// `Jikan\Request\Manga\MangaRecommendationsRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaRecommendationsRequest {
    pub id: i64,
}

impl MangaRecommendationsRequest {
    pub fn new(id: i64) -> Self {
        MangaRecommendationsRequest { id }
    }
}

impl MalRequest for MangaRecommendationsRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/{}/jikan/userrecs", self.id)
    }
}

/// `Jikan\Request\Manga\MangaReviewsRequest`.
///
/// `Constants::REVIEWS_SORT_MOST_VOTED` is `mostvoted`; spoilers and
/// preliminary default to `true` (serialized as `on`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaReviewsRequest {
    pub id: i64,
    pub page: u64,
    pub sort: String,
    pub spoilers: bool,
    pub preliminary: bool,
}

impl MangaReviewsRequest {
    pub fn new(id: i64) -> Self {
        MangaReviewsRequest {
            id,
            page: 1,
            sort: "mostvoted".to_string(),
            spoilers: true,
            preliminary: true,
        }
    }

    pub fn with_params(
        id: i64,
        page: Option<u64>,
        sort: Option<&str>,
        spoilers: Option<bool>,
        preliminary: Option<bool>,
    ) -> Self {
        MangaReviewsRequest {
            id,
            page: page.unwrap_or(1),
            sort: sort.unwrap_or("mostvoted").to_string(),
            spoilers: spoilers.unwrap_or(true),
            preliminary: preliminary.unwrap_or(true),
        }
    }
}

impl MalRequest for MangaReviewsRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            (
                "spoiler",
                QueryValue::Str(if self.spoilers { "on" } else { "off" }),
            ),
            (
                "preliminary",
                QueryValue::Str(if self.preliminary { "on" } else { "off" }),
            ),
            ("sort", QueryValue::Str(&self.sort)),
            ("p", QueryValue::UInt(self.page)),
        ]);
        format!("{BASE_URL}/manga/{}/jikan/reviews?{query}", self.id)
    }
}

/// `Jikan\Request\Manga\MangaRecentlyUpdatedByUsersRequest`.
///
/// The PHP constructor stores `($page - 1) * 75`, so `page` here is the
/// 1-based page from the API and the URL carries the 0-based offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaRecentlyUpdatedByUsersRequest {
    pub id: i64,
    pub offset: i64,
}

impl MangaRecentlyUpdatedByUsersRequest {
    pub fn new(id: i64, page: u64) -> Self {
        MangaRecentlyUpdatedByUsersRequest {
            id,
            offset: (page.saturating_sub(1) as i64) * 75,
        }
    }
}

impl MalRequest for MangaRecentlyUpdatedByUsersRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/manga/{}/jikan/stats?show={}",
            self.id, self.offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            MangaRequest::new(11).path(),
            "https://myanimelist.net/manga/11/"
        );
        assert_eq!(
            MangaCharactersRequest::new(2).path(),
            "https://myanimelist.net/manga/2/_/characters"
        );
        assert_eq!(
            MangaPicturesRequest::new(11).path(),
            "https://myanimelist.net/manga/11/jikan/pics"
        );
        assert_eq!(
            MangaStatsRequest::new(99314).path(),
            "https://myanimelist.net/manga/99314/jikan/stats"
        );
        assert_eq!(
            MangaMoreInfoRequest::new(11).path(),
            "https://myanimelist.net/manga/11/_/moreinfo"
        );
        assert_eq!(
            MangaNewsRequest::new(11).path(),
            "https://myanimelist.net/manga/11/_/news?p=1"
        );
        assert_eq!(
            MangaNewsRequest::with_page(11, Some(3)).path(),
            "https://myanimelist.net/manga/11/_/news?p=3"
        );
        // PHP: an explicit null page formats with %d -> 0.
        assert_eq!(
            MangaNewsRequest::with_page(11, None).path(),
            "https://myanimelist.net/manga/11/_/news?p=0"
        );
        assert_eq!(
            MangaForumRequest::new(11).path(),
            "https://myanimelist.net/manga/11/_/forum"
        );
        assert_eq!(
            MangaForumRequest::with_topic(11, Some("episode".into())).path(),
            "https://myanimelist.net/manga/11/_/forum?topic=episode"
        );
        // Invalid topics are dropped entirely (strict in_array check).
        assert_eq!(
            MangaForumRequest::with_topic(11, Some("bogus".into())).path(),
            "https://myanimelist.net/manga/11/_/forum"
        );
        assert_eq!(
            MangaRecommendationsRequest::new(1).path(),
            "https://myanimelist.net/manga/1/jikan/userrecs"
        );
        assert_eq!(
            MangaReviewsRequest::new(1).path(),
            "https://myanimelist.net/manga/1/jikan/reviews?spoiler=on&preliminary=on&sort=mostvoted&p=1"
        );
        assert_eq!(
            MangaReviewsRequest::with_params(1, Some(2), Some("newest"), Some(false), Some(false))
                .path(),
            "https://myanimelist.net/manga/1/jikan/reviews?spoiler=off&preliminary=off&sort=newest&p=2"
        );
        assert_eq!(
            MangaRecentlyUpdatedByUsersRequest::new(1, 1).path(),
            "https://myanimelist.net/manga/1/jikan/stats?show=0"
        );
        assert_eq!(
            MangaRecentlyUpdatedByUsersRequest::new(1, 2).path(),
            "https://myanimelist.net/manga/1/jikan/stats?show=75"
        );
    }
}
