//! `api/seasonal.rs`: the frozen seasonal entry point.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::seasonal::SeasonalParser;
use crate::request::seasonal::SeasonalRequest;
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getSeasonal(SeasonalRequest $request)`.
///
/// `later = true` requests `/anime/season/later`; the year/season are ignored
/// (PHP keeps the constructor validation).
pub async fn get_seasonal(
    client: &MalClient,
    year: u32,
    season: &str,
    later: bool,
) -> Result<Value, MalError> {
    let request = SeasonalRequest::new(Some(year), Some(season), later)
        .map_err(|error| parse_failed("SeasonalRequest", error))?;
    let path = request.path();
    let doc = client.get_html(&path).await?;
    SeasonalParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
