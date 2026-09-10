//! `api/recommendations.rs`: the frozen recent-recommendations entry point.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::recommendations::RecentRecommendationsParser;
use crate::request::recommendations::RecentRecommendationsRequest;
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getRecentRecommendations(RecentRecommendationsRequest $request)`.
pub async fn get_recent_recommendations(
    client: &MalClient,
    r#type: &str,
    page: Option<u64>,
) -> Result<Value, MalError> {
    let request = RecentRecommendationsRequest::new(r#type, page.unwrap_or(1))
        .map_err(|error| parse_failed("RecentRecommendationsRequest", error))?;
    let path = request.path();
    let doc = client.get_html(&path).await?;
    RecentRecommendationsParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
