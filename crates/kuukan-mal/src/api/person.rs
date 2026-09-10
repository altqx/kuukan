//! Frozen person entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::person::PersonParser;
use crate::request::person::{PersonPicturesRequest, PersonRequest};
use crate::request::MalRequest;

/// `MalClient::getPerson()`.
pub async fn get_person(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = PersonRequest::new(id).path();
    if id == 0 {
        return Err(MalError::BadResponse { status: 404, url: path });
    }
    let doc = client.get_html(&path).await?;
    PersonParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}

/// `MalClient::getPersonPictures()` — array of `PersonImageResource`.
pub async fn get_person_pictures(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = PersonPicturesRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    crate::parser::common::default_pictures_page(&doc)
        .map(Value::Array)
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}
