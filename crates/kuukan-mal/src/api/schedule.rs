//! `api/schedule.rs`: the frozen schedule entry point.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::schedule::ScheduleParser;
use crate::request::schedule::ScheduleRequest;
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getSchedule(ScheduleRequest $request)`.
pub async fn get_schedule(client: &MalClient) -> Result<Value, MalError> {
    let path = ScheduleRequest::new().path();
    let doc = client.get_html(&path).await?;
    ScheduleParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
