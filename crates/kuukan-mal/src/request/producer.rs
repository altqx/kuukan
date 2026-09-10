//! Port of `Jikan\Request\Producer\*` (jikan-php v4.0.12).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Producer\ProducerRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerRequest {
    pub id: i64,
    pub page: u64,
}

impl ProducerRequest {
    pub fn new(id: i64, page: u64) -> Self {
        ProducerRequest { id, page }
    }
}

impl MalRequest for ProducerRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/producer/{}?page={}", self.id, self.page)
    }
}

/// `Jikan\Request\Producer\ProducersRequest`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProducersRequest;

impl ProducersRequest {
    pub fn new() -> Self {
        ProducersRequest
    }
}

impl MalRequest for ProducersRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/producer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            ProducerRequest::new(1, 1).path(),
            "https://myanimelist.net/anime/producer/1?page=1"
        );
        assert_eq!(
            ProducerRequest::new(1, 3).path(),
            "https://myanimelist.net/anime/producer/1?page=3"
        );
        assert_eq!(
            ProducersRequest::new().path(),
            "https://myanimelist.net/anime/producer"
        );
    }
}
