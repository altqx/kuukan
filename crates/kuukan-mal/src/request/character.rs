//! Port of `Jikan\Request\Character\*` (jikan-php v4.0.12).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Character\CharacterRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRequest {
    pub id: i64,
}

impl CharacterRequest {
    pub fn new(id: i64) -> Self {
        CharacterRequest { id }
    }
}

impl MalRequest for CharacterRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/character/{}", self.id)
    }
}

/// `Jikan\Request\Character\CharacterPicturesRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPicturesRequest {
    pub id: i64,
}

impl CharacterPicturesRequest {
    pub fn new(id: i64) -> Self {
        CharacterPicturesRequest { id }
    }
}

impl MalRequest for CharacterPicturesRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/character/{}/jikan/pics", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            CharacterRequest::new(116281).path(),
            "https://myanimelist.net/character/116281"
        );
        assert_eq!(
            CharacterPicturesRequest::new(105591).path(),
            "https://myanimelist.net/character/105591/jikan/pics"
        );
    }
}
