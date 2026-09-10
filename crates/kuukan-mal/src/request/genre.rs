//! Port of `Jikan\Request\Genre\*` (jikan-php v4.0.12).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Genre\AnimeGenreRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimeGenreRequest {
    pub id: i64,
    pub page: u64,
}

impl AnimeGenreRequest {
    pub fn new(id: i64, page: u64) -> Self {
        AnimeGenreRequest { id, page }
    }
}

impl MalRequest for AnimeGenreRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/genre/{}?page={}", self.id, self.page)
    }
}

/// `Jikan\Request\Genre\AnimeGenresRequest`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnimeGenresRequest;

impl AnimeGenresRequest {
    pub fn new() -> Self {
        AnimeGenresRequest
    }
}

impl MalRequest for AnimeGenresRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime.php")
    }
}

/// `Jikan\Request\Genre\MangaGenreRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaGenreRequest {
    pub id: i64,
    pub page: u64,
}

impl MangaGenreRequest {
    pub fn new(id: i64, page: u64) -> Self {
        MangaGenreRequest { id, page }
    }
}

impl MalRequest for MangaGenreRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/genre/{}?page={}", self.id, self.page)
    }
}

/// `Jikan\Request\Genre\MangaGenresRequest`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MangaGenresRequest;

impl MangaGenresRequest {
    pub fn new() -> Self {
        MangaGenresRequest
    }
}

impl MalRequest for MangaGenresRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga.php")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            AnimeGenreRequest::new(1, 1).path(),
            "https://myanimelist.net/anime/genre/1?page=1"
        );
        assert_eq!(
            AnimeGenresRequest::new().path(),
            "https://myanimelist.net/anime.php"
        );
        assert_eq!(
            MangaGenreRequest::new(1, 1).path(),
            "https://myanimelist.net/manga/genre/1?page=1"
        );
        assert_eq!(
            MangaGenresRequest::new().path(),
            "https://myanimelist.net/manga.php"
        );
    }
}
