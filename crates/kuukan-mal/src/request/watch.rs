//! Ports of `Jikan\Request\Watch\*`.

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Watch\RecentEpisodesRequest`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecentEpisodesRequest;

impl RecentEpisodesRequest {
    pub fn new() -> Self {
        RecentEpisodesRequest
    }
}

impl MalRequest for RecentEpisodesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/watch/episode")
    }
}

/// `Jikan\Request\Watch\PopularEpisodesRequest`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PopularEpisodesRequest;

impl PopularEpisodesRequest {
    pub fn new() -> Self {
        PopularEpisodesRequest
    }
}

impl MalRequest for PopularEpisodesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/watch/episode/popular")
    }
}

/// `Jikan\Request\Watch\RecentPromotionalVideosRequest`.
#[derive(Debug, Clone, Copy)]
pub struct RecentPromotionalVideosRequest {
    page: u64,
}

impl RecentPromotionalVideosRequest {
    /// `new RecentPromotionalVideosRequest($page = 1)`.
    pub fn new(page: u64) -> Self {
        RecentPromotionalVideosRequest { page }
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }
}

impl MalRequest for RecentPromotionalVideosRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/watch/promotion?p={}", self.page)
    }
}

/// `Jikan\Request\Watch\PopularPromotionalVideosRequest`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PopularPromotionalVideosRequest;

impl PopularPromotionalVideosRequest {
    pub fn new() -> Self {
        PopularPromotionalVideosRequest
    }
}

impl MalRequest for PopularPromotionalVideosRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/watch/promotion/popular")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php() {
        assert_eq!(
            RecentEpisodesRequest::new().path(),
            "https://myanimelist.net/watch/episode"
        );
        assert_eq!(
            PopularEpisodesRequest::new().path(),
            "https://myanimelist.net/watch/episode/popular"
        );
        assert_eq!(
            RecentPromotionalVideosRequest::new(2).path(),
            "https://myanimelist.net/watch/promotion?p=2"
        );
        assert_eq!(
            PopularPromotionalVideosRequest::new().path(),
            "https://myanimelist.net/watch/promotion/popular"
        );
    }
}
