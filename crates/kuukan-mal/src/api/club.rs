//! Frozen club entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::club::{ClubParser, UserListParser};
use crate::request::club::{ClubRequest, UserListRequest};
use crate::source::MalSource;

/// `MalClient::getClub()`.
pub async fn get_club(client: &dyn MalSource, id: i64) -> Result<Value, MalError> {
    super::fetch_then(client, ClubRequest::new(id), |doc| {
        ClubParser::new(doc).model()
    })
    .await
}

/// `MalClient::getClubUsers()` — page starts at 1.
pub async fn get_club_users(client: &dyn MalSource, id: i64, page: u64) -> Result<Value, MalError> {
    super::fetch_then(client, UserListRequest::new(id, page), |doc| {
        UserListParser::new(doc).model()
    })
    .await
}
