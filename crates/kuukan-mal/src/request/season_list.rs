//! Port of `Jikan\Request\SeasonList\SeasonListRequest`.

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\SeasonList\SeasonListRequest`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SeasonListRequest;

impl SeasonListRequest {
    pub fn new() -> Self {
        SeasonListRequest
    }
}

impl MalRequest for SeasonListRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/anime/season/archive")
    }
}
