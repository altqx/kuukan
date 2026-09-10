//! Anime request URL builders: port of `Jikan\Request\Anime\*` plus the two
//! anime-genre requests whose API entry points live in `api::anime`.
//!
//! Each builder mirrors the PHP class' `getPath()` byte for byte, including the
//! computed offsets (`(page - 1) * 100` for episodes, `(page - 1) * 75` for the
//! recently-updated list) and the `http_build_query` encoding.

use crate::request::{http_build_query, MalRequest, QueryValue};

/// `Constants::BASE_URL`.
const BASE_URL: &str = "https://myanimelist.net";

/// `AnimeForumRequest::$validTypes`.
pub const ANIME_FORUM_TYPES: [&str; 3] = ["all", "episode", "other"];

/// `Jikan\Request\Anime\AnimeRequest`.
#[derive(Debug, Clone)]
pub struct AnimeRequest {
    id: i64,
}

impl AnimeRequest {
    pub fn new(id: i64) -> Self {
        AnimeRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimeRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/", self.id)
    }
}

/// `Jikan\Request\Anime\AnimeEpisodesRequest`.
#[derive(Debug, Clone)]
pub struct AnimeEpisodesRequest {
    id: i64,
    /// `($page - 1) * 100`.
    offset: i64,
}

impl AnimeEpisodesRequest {
    pub fn new(id: i64, page: i64) -> Self {
        AnimeEpisodesRequest {
            id,
            offset: (page - 1) * 100,
        }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_page(&self) -> i64 {
        self.offset
    }
}

impl MalRequest for AnimeEpisodesRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/anime/{}/_/episode?offset={}",
            self.id, self.offset
        )
    }
}

/// `Jikan\Request\Anime\AnimeEpisodeRequest`.
#[derive(Debug, Clone)]
pub struct AnimeEpisodeRequest {
    id: i64,
    episode_id: i64,
}

impl AnimeEpisodeRequest {
    pub fn new(id: i64, episode_id: i64) -> Self {
        AnimeEpisodeRequest { id, episode_id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_episode_id(&self) -> i64 {
        self.episode_id
    }
}

impl MalRequest for AnimeEpisodeRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/_/episode/{}", self.id, self.episode_id)
    }
}

/// `Jikan\Request\Anime\AnimeVideosRequest`.
#[derive(Debug, Clone)]
pub struct AnimeVideosRequest {
    id: i64,
}

impl AnimeVideosRequest {
    pub fn new(id: i64) -> Self {
        AnimeVideosRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimeVideosRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/_/video", self.id)
    }
}

/// `Jikan\Request\Anime\AnimeVideosEpisodesRequest`.
#[derive(Debug, Clone)]
pub struct AnimeVideosEpisodesRequest {
    id: i64,
    page: i64,
}

impl AnimeVideosEpisodesRequest {
    pub fn new(id: i64, page: i64) -> Self {
        AnimeVideosEpisodesRequest { id, page }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_page(&self) -> i64 {
        self.page
    }
}

impl MalRequest for AnimeVideosEpisodesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/_/video?p={}", self.id, self.page)
    }
}

/// `Jikan\Request\Anime\AnimeCharactersAndStaffRequest`.
#[derive(Debug, Clone)]
pub struct AnimeCharactersAndStaffRequest {
    id: i64,
}

impl AnimeCharactersAndStaffRequest {
    pub fn new(id: i64) -> Self {
        AnimeCharactersAndStaffRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimeCharactersAndStaffRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/_/characters", self.id)
    }
}

/// `Jikan\Request\Anime\AnimePicturesRequest`.
#[derive(Debug, Clone)]
pub struct AnimePicturesRequest {
    id: i64,
}

impl AnimePicturesRequest {
    pub fn new(id: i64) -> Self {
        AnimePicturesRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimePicturesRequest {
    fn path(&self) -> String {
        // MyAnimeList wants <something> after /<id>/...; it accepts `jikan`.
        format!("{BASE_URL}/anime/{}/jikan/pics", self.id)
    }
}

/// `Jikan\Request\Anime\AnimeMoreInfoRequest`.
#[derive(Debug, Clone)]
pub struct AnimeMoreInfoRequest {
    id: i64,
}

impl AnimeMoreInfoRequest {
    pub fn new(id: i64) -> Self {
        AnimeMoreInfoRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimeMoreInfoRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/_/moreinfo", self.id)
    }
}

/// `Jikan\Request\Anime\AnimeStatsRequest`.
#[derive(Debug, Clone)]
pub struct AnimeStatsRequest {
    id: i64,
}

impl AnimeStatsRequest {
    pub fn new(id: i64) -> Self {
        AnimeStatsRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimeStatsRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/jikan/stats", self.id)
    }
}

/// `Jikan\Request\Anime\AnimeForumRequest`.
#[derive(Debug, Clone)]
pub struct AnimeForumRequest {
    id: i64,
    topic: Option<String>,
}

impl AnimeForumRequest {
    pub fn new(id: i64, topic: Option<&str>) -> Self {
        AnimeForumRequest {
            id,
            topic: topic.map(str::to_string),
        }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_topic(&self) -> Option<&str> {
        self.topic.as_deref()
    }

    pub fn get_valid_types() -> [&'static str; 3] {
        ANIME_FORUM_TYPES
    }
}

impl MalRequest for AnimeForumRequest {
    fn path(&self) -> String {
        let query = match &self.topic {
            Some(topic) if ANIME_FORUM_TYPES.contains(&topic.as_str()) => format!(
                "?{}",
                http_build_query(&[("topic", QueryValue::Str(topic))])
            ),
            _ => String::new(),
        };
        format!("{BASE_URL}/anime/{}/_/forum{}", self.id, query)
    }
}

/// `Jikan\Request\Anime\AnimeNewsRequest`.
#[derive(Debug, Clone)]
pub struct AnimeNewsRequest {
    id: i64,
    page: i64,
}

impl AnimeNewsRequest {
    pub fn new(id: i64, page: i64) -> Self {
        AnimeNewsRequest { id, page }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_page(&self) -> i64 {
        self.page
    }
}

impl MalRequest for AnimeNewsRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/_/news?p={}", self.id, self.page)
    }
}

/// `Jikan\Request\Anime\AnimeRecentlyUpdatedByUsersRequest`.
#[derive(Debug, Clone)]
pub struct AnimeRecentlyUpdatedByUsersRequest {
    id: i64,
    /// `($page - 1) * 75`.
    offset: i64,
}

impl AnimeRecentlyUpdatedByUsersRequest {
    pub fn new(id: i64, page: i64) -> Self {
        AnimeRecentlyUpdatedByUsersRequest {
            id,
            offset: (page - 1) * 75,
        }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_page(&self) -> i64 {
        self.offset
    }
}

impl MalRequest for AnimeRecentlyUpdatedByUsersRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/anime/{}/jikan/stats?show={}",
            self.id, self.offset
        )
    }
}

/// `Jikan\Request\Anime\AnimeRecommendationsRequest`.
#[derive(Debug, Clone)]
pub struct AnimeRecommendationsRequest {
    id: i64,
}

impl AnimeRecommendationsRequest {
    pub fn new(id: i64) -> Self {
        AnimeRecommendationsRequest { id }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }
}

impl MalRequest for AnimeRecommendationsRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/{}/jikan/userrecs", self.id)
    }
}

/// `Constants::REVIEWS_SORT_MOST_VOTED`.
pub const REVIEWS_SORT_MOST_VOTED: &str = "mostvoted";
/// `Constants::REVIEWS_SORT_OLDEST`.
pub const REVIEWS_SORT_OLDEST: &str = "oldest";
/// `Constants::REVIEWS_SORT_NEWEST`.
pub const REVIEWS_SORT_NEWEST: &str = "newest";

/// `Jikan\Request\Anime\AnimeReviewsRequest`.
#[derive(Debug, Clone)]
pub struct AnimeReviewsRequest {
    id: i64,
    page: i64,
    sort: String,
    spoilers: bool,
    preliminary: bool,
}

impl AnimeReviewsRequest {
    pub fn new(id: i64, page: i64, sort: &str, spoilers: bool, preliminary: bool) -> Self {
        AnimeReviewsRequest {
            id,
            page,
            sort: sort.to_string(),
            spoilers,
            preliminary,
        }
    }

    pub fn get_id(&self) -> i64 {
        self.id
    }

    pub fn get_page(&self) -> i64 {
        self.page
    }

    pub fn get_sort(&self) -> &str {
        &self.sort
    }

    pub fn is_spoilers(&self) -> bool {
        self.spoilers
    }

    pub fn is_preliminary(&self) -> bool {
        self.preliminary
    }
}

impl MalRequest for AnimeReviewsRequest {
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
            ("p", QueryValue::Int(self.page)),
        ]);
        format!("{BASE_URL}/anime/{}/jikan/reviews?{query}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected paths ported from the PHP `getPath()` methods.
    #[test]
    fn paths_match_php() {
        assert_eq!(
            AnimeRequest::new(6).path(),
            "https://myanimelist.net/anime/6/"
        );
        assert_eq!(
            AnimeEpisodesRequest::new(21, 1).path(),
            "https://myanimelist.net/anime/21/_/episode?offset=0"
        );
        assert_eq!(
            AnimeEpisodesRequest::new(21, 11).path(),
            "https://myanimelist.net/anime/21/_/episode?offset=1000"
        );
        assert_eq!(
            AnimeEpisodeRequest::new(21, 1).path(),
            "https://myanimelist.net/anime/21/_/episode/1"
        );
        assert_eq!(
            AnimeVideosRequest::new(1).path(),
            "https://myanimelist.net/anime/1/_/video"
        );
        assert_eq!(
            AnimeVideosEpisodesRequest::new(1, 2).path(),
            "https://myanimelist.net/anime/1/_/video?p=2"
        );
        assert_eq!(
            AnimeCharactersAndStaffRequest::new(1).path(),
            "https://myanimelist.net/anime/1/_/characters"
        );
        assert_eq!(
            AnimePicturesRequest::new(22147).path(),
            "https://myanimelist.net/anime/22147/jikan/pics"
        );
        assert_eq!(
            AnimeMoreInfoRequest::new(21).path(),
            "https://myanimelist.net/anime/21/_/moreinfo"
        );
        assert_eq!(
            AnimeStatsRequest::new(37405).path(),
            "https://myanimelist.net/anime/37405/jikan/stats"
        );
        assert_eq!(
            AnimeNewsRequest::new(1, 2).path(),
            "https://myanimelist.net/anime/1/_/news?p=2"
        );
        assert_eq!(
            AnimeRecentlyUpdatedByUsersRequest::new(1, 1).path(),
            "https://myanimelist.net/anime/1/jikan/stats?show=0"
        );
        assert_eq!(
            AnimeRecentlyUpdatedByUsersRequest::new(1, 2).path(),
            "https://myanimelist.net/anime/1/jikan/stats?show=75"
        );
        assert_eq!(
            AnimeRecommendationsRequest::new(21).path(),
            "https://myanimelist.net/anime/21/jikan/userrecs"
        );
    }

    #[test]
    fn forum_path_validation() {
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
    }

    #[test]
    fn reviews_path_matches_php() {
        assert_eq!(
            AnimeReviewsRequest::new(1, 1, REVIEWS_SORT_MOST_VOTED, true, true).path(),
            "https://myanimelist.net/anime/1/jikan/reviews?spoiler=on&preliminary=on&sort=mostvoted&p=1"
        );
        assert_eq!(
            AnimeReviewsRequest::new(5, 3, REVIEWS_SORT_NEWEST, false, false).path(),
            "https://myanimelist.net/anime/5/jikan/reviews?spoiler=off&preliminary=off&sort=newest&p=3"
        );
    }
}
