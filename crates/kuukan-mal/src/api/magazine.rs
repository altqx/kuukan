//! Frozen magazine entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::magazine::{MagazineListParser, MagazineParser};
use crate::request::magazine::{MagazineRequest, MagazinesRequest};
use crate::source::MalSource;

/// `MalClient::getMagazine()` — `/manga/magazine/{id}`.
pub async fn get_magazine(client: &dyn MalSource, id: i64, page: u64) -> Result<Value, MalError> {
    super::fetch_then(client, MagazineRequest::new(id, page), |doc| {
        MagazineParser::new(doc).model()
    })
    .await
}

/// `MalClient::getMagazines()` — full magazine list.
pub async fn get_magazines(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_then(client, MagazinesRequest::new(), |doc| {
        MagazineListParser::new(doc).model()
    })
    .await
}
