//! Port of `Jikan\Request\Person\*` (jikan-php v4.0.12).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Person\PersonRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonRequest {
    pub id: i64,
}

impl PersonRequest {
    pub fn new(id: i64) -> Self {
        PersonRequest { id }
    }
}

impl MalRequest for PersonRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/people/{}", self.id)
    }
}

/// `Jikan\Request\Person\PersonPicturesRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonPicturesRequest {
    pub id: i64,
}

impl PersonPicturesRequest {
    pub fn new(id: i64) -> Self {
        PersonPicturesRequest { id }
    }
}

impl MalRequest for PersonPicturesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/people/{}/jikan/pics", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            PersonRequest::new(99).path(),
            "https://myanimelist.net/people/99"
        );
        assert_eq!(
            PersonPicturesRequest::new(11162).path(),
            "https://myanimelist.net/people/11162/jikan/pics"
        );
    }
}
