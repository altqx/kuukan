//! Frozen person entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::person::PersonParser;
use crate::request::person::{PersonPicturesRequest, PersonRequest};
use crate::request::MalRequest;
use crate::source::{MalSource, MalSourceExt};

/// `MalClient::getPerson()`.
pub async fn get_person(client: &dyn MalSource, id: i64) -> Result<Value, MalError> {
    let path = PersonRequest::new(id).path();
    if id == 0 {
        return Err(MalError::BadResponse {
            status: 404,
            url: path,
        });
    }
    let doc = client.get_html(&path).await?;
    PersonParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}

/// `MalClient::getPersonPictures()` — array of `PersonImageResource`.
pub async fn get_person_pictures(client: &dyn MalSource, id: i64) -> Result<Value, MalError> {
    super::fetch_then(client, PersonPicturesRequest::new(id), |doc| {
        crate::parser::common::default_pictures_page(&doc).map(Value::Array)
    })
    .await
}
