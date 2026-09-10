//! Port of `Jikan\Request\Club\*` (jikan-php v4.0.12).

use crate::request::{MalRequest, BASE_URL};

/// `Jikan\Request\Club\ClubRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubRequest {
    pub club_id: i64,
}

impl ClubRequest {
    pub fn new(club_id: i64) -> Self {
        ClubRequest { club_id }
    }
}

impl MalRequest for ClubRequest {
    fn path(&self) -> String {
        format!("{BASE_URL}/clubs.php?cid={}", self.club_id)
    }
}

/// `Jikan\Request\Club\UserListRequest`.
///
/// The PHP constructor stores `($page - 1) * 36`, so `page` here is the
/// 1-based page and the URL carries the 0-based offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserListRequest {
    pub club_id: i64,
    pub offset: i64,
}

impl UserListRequest {
    pub fn new(club_id: i64, page: u64) -> Self {
        UserListRequest {
            club_id,
            offset: (page.saturating_sub(1) as i64) * 36,
        }
    }
}

impl MalRequest for UserListRequest {
    fn path(&self) -> String {
        format!(
            "{BASE_URL}/clubs.php?action=view&t=members&id={}&show={}",
            self.club_id, self.offset
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_match_php_get_path() {
        assert_eq!(
            ClubRequest::new(1).path(),
            "https://myanimelist.net/clubs.php?cid=1"
        );
        assert_eq!(
            UserListRequest::new(21349, 1).path(),
            "https://myanimelist.net/clubs.php?action=view&t=members&id=21349&show=0"
        );
        assert_eq!(
            UserListRequest::new(21349, 2).path(),
            "https://myanimelist.net/clubs.php?action=view&t=members&id=21349&show=36"
        );
    }
}
