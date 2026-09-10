//! Port of `Jikan\Request\Magazine\*` (jikan-php v4.0.12).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Magazine\MagazineRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagazineRequest {
    pub id: i64,
    pub page: u64,
}

impl MagazineRequest {
    pub fn new(id: i64, page: u64) -> Self {
        MagazineRequest { id, page }
    }
}

impl MalRequest for MagazineRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/magazine/{}?page={}", self.id, self.page)
    }
}

/// `Jikan\Request\Magazine\MagazinesRequest`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MagazinesRequest;

impl MagazinesRequest {
    pub fn new() -> Self {
        MagazinesRequest
    }
}

impl MalRequest for MagazinesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/manga/magazine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            MagazineRequest::new(1, 1).path(),
            "https://myanimelist.net/manga/magazine/1?page=1"
        );
        assert_eq!(
            MagazinesRequest::new().path(),
            "https://myanimelist.net/manga/magazine"
        );
    }
}
