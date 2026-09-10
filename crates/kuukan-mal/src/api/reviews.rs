//! `api/reviews.rs`: the frozen global-reviews entry point from

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::reviews::ReviewsParser;
use crate::request::reviews::ReviewsRequest;
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getReviews(ReviewsRequest $request)` (global reviews).
pub async fn get_reviews(
    client: &MalClient,
    r#type: &str,
    page: Option<u64>,
    sort: &str,
    spoilers: bool,
    preliminary: bool,
) -> Result<Value, MalError> {
    let request = ReviewsRequest::new(r#type, page.unwrap_or(1), sort, spoilers, preliminary)
        .map_err(|error| parse_failed("ReviewsRequest", error))?;
    let path = request.path();
    let doc = client.get_html(&path).await?;
    ReviewsParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
