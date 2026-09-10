//! Ports of `Jikan\Request\Top\*`.

use crate::request::{http_build_query, MalRequest, QueryValue, BASE_URL};

/// `Constants::TOP_AIRING` .. `Constants::TOP_ONA` + shared popularity keys.
pub const VALID_ANIME_TYPES: [&str; 9] = [
    "airing",
    "upcoming",
    "tv",
    "movie",
    "ova",
    "special",
    "bypopularity",
    "favorite",
    "ona",
];

/// `Constants::TOP_MANGA` .. `Constants::TOP_LIGHTNOVELS` + shared keys.
pub const VALID_MANGA_TYPES: [&str; 9] = [
    "manga",
    "novels",
    "oneshots",
    "doujin",
    "manhwa",
    "manhua",
    "bypopularity",
    "favorite",
    "lightnovels",
];

/// `Jikan\Request\Top\TopAnimeRequest`.
#[derive(Debug, Clone)]
pub struct TopAnimeRequest {
    page: u64,
    type_: Option<String>,
}

impl TopAnimeRequest {
    /// `new TopAnimeRequest($page = 1, $type = null)`.
    pub fn new(page: u64, type_: Option<&str>) -> Result<Self, String> {
        if let Some(type_) = type_ {
            if !VALID_ANIME_TYPES.contains(&type_) {
                return Err(format!("Type {type_} is not valid"));
            }
        }
        Ok(TopAnimeRequest {
            page,
            type_: type_.map(|t| t.to_string()),
        })
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }

    /// `getType()`.
    pub fn type_(&self) -> Option<&str> {
        self.type_.as_deref()
    }
}

impl MalRequest for TopAnimeRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("limit", QueryValue::UInt(50 * (self.page - 1))),
            ("type", self.type_.as_deref().into()),
        ]);
        format!("{BASE_URL}/topanime.php?{query}")
    }
}

/// `Jikan\Request\Top\TopMangaRequest`.
#[derive(Debug, Clone)]
pub struct TopMangaRequest {
    page: u64,
    type_: Option<String>,
}

impl TopMangaRequest {
    /// `new TopMangaRequest($page = 1, $type = null)`.
    pub fn new(page: u64, type_: Option<&str>) -> Result<Self, String> {
        if let Some(type_) = type_ {
            if !VALID_MANGA_TYPES.contains(&type_) {
                return Err(format!("Type {type_} is not valid"));
            }
        }
        Ok(TopMangaRequest {
            page,
            type_: type_.map(|t| t.to_string()),
        })
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }

    /// `getType()`.
    pub fn type_(&self) -> Option<&str> {
        self.type_.as_deref()
    }
}

impl MalRequest for TopMangaRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[
            ("limit", QueryValue::UInt(50 * (self.page - 1))),
            ("type", self.type_.as_deref().into()),
        ]);
        format!("{BASE_URL}/topmanga.php?{query}")
    }
}

/// `Jikan\Request\Top\TopCharactersRequest`.
#[derive(Debug, Clone, Copy)]
pub struct TopCharactersRequest {
    page: u64,
}

impl TopCharactersRequest {
    /// `new TopCharactersRequest($page = 1)`.
    pub fn new(page: u64) -> Self {
        TopCharactersRequest { page }
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }
}

impl MalRequest for TopCharactersRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[("limit", QueryValue::UInt(50 * (self.page - 1)))]);
        format!("{BASE_URL}/character.php?{query}")
    }
}

/// `Jikan\Request\Top\TopPeopleRequest`.
#[derive(Debug, Clone, Copy)]
pub struct TopPeopleRequest {
    page: u64,
}

impl TopPeopleRequest {
    /// `new TopPeopleRequest($page = 1)`.
    pub fn new(page: u64) -> Self {
        TopPeopleRequest { page }
    }

    /// `getPage()`.
    pub fn page(&self) -> u64 {
        self.page
    }
}

impl MalRequest for TopPeopleRequest {
    fn path(&self) -> String {
        let query = http_build_query(&[("limit", QueryValue::UInt(50 * (self.page - 1)))]);
        format!("{BASE_URL}/people.php?{query}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php() {
        assert_eq!(
            TopAnimeRequest::new(1, None).unwrap().path(),
            "https://myanimelist.net/topanime.php?limit=0"
        );
        assert_eq!(
            TopAnimeRequest::new(2, Some("airing")).unwrap().path(),
            "https://myanimelist.net/topanime.php?limit=50&type=airing"
        );
        assert_eq!(
            TopMangaRequest::new(1, Some("manga")).unwrap().path(),
            "https://myanimelist.net/topmanga.php?limit=0&type=manga"
        );
        assert_eq!(
            TopCharactersRequest::new(3).path(),
            "https://myanimelist.net/character.php?limit=100"
        );
        assert_eq!(
            TopPeopleRequest::new(1).path(),
            "https://myanimelist.net/people.php?limit=0"
        );
        assert!(TopAnimeRequest::new(1, Some("bogus")).is_err());
        assert!(TopMangaRequest::new(1, Some("bogus")).is_err());
    }
}
