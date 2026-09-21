//! `api/schedule.rs`: the frozen schedule entry point.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::schedule::ScheduleParser;
use crate::request::schedule::ScheduleRequest;
use crate::source::MalSource;

/// `MalClient::getSchedule(ScheduleRequest $request)`.
pub async fn get_schedule(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<ScheduleParser>(client, ScheduleRequest::new()).await
}
