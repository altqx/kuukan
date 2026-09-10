//! `api/season_list.rs`: the frozen season-list entry point from

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::season_list::SeasonListParser;
use crate::request::season_list::SeasonListRequest;
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getSeasonList(SeasonListRequest $request)`.
pub async fn get_season_list(client: &MalClient) -> Result<Value, MalError> {
    let path = SeasonListRequest::new().path();
    let doc = client.get_html(&path).await?;
    SeasonListParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
