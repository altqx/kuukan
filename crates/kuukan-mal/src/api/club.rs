//! Frozen club entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::club::{ClubParser, UserListParser};
use crate::request::club::{ClubRequest, UserListRequest};
use crate::request::MalRequest;

/// `MalClient::getClub()`.
pub async fn get_club(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = ClubRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    ClubParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}

/// `MalClient::getClubUsers()` — page starts at 1.
pub async fn get_club_users(client: &MalClient, id: i64, page: u64) -> Result<Value, MalError> {
    let path = UserListRequest::new(id, page).path();
    let doc = client.get_html(&path).await?;
    UserListParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}
