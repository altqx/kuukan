//! Frozen magazine entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::magazine::{MagazineListParser, MagazineParser};
use crate::request::magazine::{MagazineRequest, MagazinesRequest};
use crate::request::MalRequest;

/// `MalClient::getMagazine()` — `/manga/magazine/{id}`.
pub async fn get_magazine(client: &MalClient, id: i64, page: u64) -> Result<Value, MalError> {
    let path = MagazineRequest::new(id, page).path();
    let doc = client.get_html(&path).await?;
    MagazineParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}

/// `MalClient::getMagazines()` — full magazine list.
pub async fn get_magazines(client: &MalClient) -> Result<Value, MalError> {
    let path = MagazinesRequest::new().path();
    let doc = client.get_html(&path).await?;
    MagazineListParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}
