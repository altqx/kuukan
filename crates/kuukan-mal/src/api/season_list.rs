//! `api/season_list.rs`: the frozen season-list entry point from

use serde_json::Value;

use crate::error::MalError;
use crate::parser::season_list::SeasonListParser;
use crate::request::season_list::SeasonListRequest;
use crate::source::MalSource;

/// `MalClient::getSeasonList(SeasonListRequest $request)`.
pub async fn get_season_list(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<SeasonListParser>(client, SeasonListRequest::new()).await
}
