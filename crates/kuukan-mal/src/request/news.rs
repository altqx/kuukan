//! News request paths shared by the anime/manga APIs.
//!
//! Ports of `Jikan\Request\Anime\AnimeNewsRequest` and
//! `Jikan\Request\Manga\MangaNewsRequest` (the news parsers live in
//! [`crate::parser::news`], the shared payload is produced by
//! [`crate::parser::news::parse_news`]).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Anime\AnimeNewsRequest`.
#[derive(Debug, Clone, Copy)]
pub struct AnimeNewsRequest {
    id: i64,
    page: Option<i64>,
}

impl AnimeNewsRequest {
    /// `new AnimeNewsRequest($id, $page = 1)`.
    pub fn new(id: i64, page: Option<i64>) -> Self {
        AnimeNewsRequest { id, page }
    }

    /// `getId()`.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// `getPage()`.
    pub fn page(&self) -> Option<i64> {
        self.page
    }
}

impl MalRequest for AnimeNewsRequest {
    fn path(&self) -> String {
        // `sprintf('%d', null)` is "0" in PHP.
        format!(
            "{BASE_URL}/anime/{}/_/news?p={}",
            self.id,
            self.page.unwrap_or(0)
        )
    }
}

/// `Jikan\Request\Manga\MangaNewsRequest`.
#[derive(Debug, Clone, Copy)]
pub struct MangaNewsRequest {
    id: i64,
    page: Option<i64>,
}

impl MangaNewsRequest {
    /// `new MangaNewsRequest($id, $page = 1)`.
    pub fn new(id: i64, page: Option<i64>) -> Self {
        MangaNewsRequest { id, page }
    }

    /// `getId()`.
    pub fn id(&self) -> i64 {
        self.id
    }

    /// `getPage()`.
    pub fn page(&self) -> Option<i64> {
        self.page
    }
}

impl MalRequest for MangaNewsRequest {
    fn path(&self) -> String {
        // `sprintf('%d', null)` is "0" in PHP.
        format!(
            "{BASE_URL}/manga/{}/_/news?p={}",
            self.id,
            self.page.unwrap_or(0)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php() {
        assert_eq!(
            AnimeNewsRequest::new(1, None).path(),
            "https://myanimelist.net/anime/1/_/news?p=0"
        );
        assert_eq!(
            AnimeNewsRequest::new(1, Some(1)).path(),
            "https://myanimelist.net/anime/1/_/news?p=1"
        );
        assert_eq!(
            AnimeNewsRequest::new(21, Some(2)).path(),
            "https://myanimelist.net/anime/21/_/news?p=2"
        );
        assert_eq!(
            MangaNewsRequest::new(2, None).path(),
            "https://myanimelist.net/manga/2/_/news?p=0"
        );
    }
}
