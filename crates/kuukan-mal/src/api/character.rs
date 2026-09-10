//! Frozen character entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::character::CharacterParser;
use crate::request::character::{CharacterPicturesRequest, CharacterRequest};
use crate::request::MalRequest;

/// `MalClient::getCharacter()`.
///
/// MAL returns `Invalid ID provided.` instead of a 404 for invalid characters,
/// so the `badresult` div becomes a 404 exactly like in PHP.
pub async fn get_character(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = CharacterRequest::new(id).path();
    if id == 0 {
        return Err(MalError::BadResponse {
            status: 404,
            url: path,
        });
    }
    let doc = client.get_html(&path).await?;
    if doc.count("//*[@id=\"content\"]/div[@class=\"badresult\"]")? > 0 {
        return Err(MalError::BadResponse {
            status: 404,
            url: path,
        });
    }
    CharacterParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}

/// `MalClient::getCharacterPictures()` — array of `PersonImageResource`.
pub async fn get_character_pictures(client: &MalClient, id: i64) -> Result<Value, MalError> {
    let path = CharacterPicturesRequest::new(id).path();
    let doc = client.get_html(&path).await?;
    crate::parser::common::default_pictures_page(&doc)
        .map(Value::Array)
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}
