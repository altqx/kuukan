//! The MyAnimeList entry points, one module per resource.
//!
//! Every function here is the same four steps — build the path, fetch it,
//! parse it, wrap a parse failure — so the steps live in [`fetch_and_parse`]
//! and each function supplies only what varies: which request, which parser.
//! The named functions stay, because the name is the locality this module
//! earns; it was only the body that was copied seventy-odd times.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::ParseModel;
use crate::request::MalRequest;
use crate::source::{MalSource, MalSourceExt};

pub mod anime;
pub mod character;
pub mod club;
pub mod genre;
pub mod magazine;
pub mod manga;
pub mod person;
pub mod producer;
pub mod recommendations;
pub mod reviews;
pub mod schedule;
pub mod search;
pub mod season_list;
pub mod seasonal;
pub mod top;
pub mod user;
pub mod watch;

/// Fetch a page and turn it into its model.
///
/// A parse failure becomes `ParserException::fromRequest()`, carrying the path
/// that produced it.
pub(crate) async fn fetch_and_parse<P: ParseModel>(
    client: &dyn MalSource,
    request: impl MalRequest,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = client.get_html(&path).await?;
    P::model(doc).map_err(|error| MalError::parse_failed(&path, error.to_string()))
}

/// Fetch a page and hand it to `parse`.
///
/// For the entry points whose parsing is not a plain [`ParseModel`] — a free
/// parser function, a list that becomes a JSON array, a parser whose model is
/// assembled from more than one call. They still get the path, the fetch and
/// the error wrapping from one place.
pub(crate) async fn fetch_then<T: Into<Value>>(
    client: &dyn MalSource,
    request: impl MalRequest,
    parse: impl FnOnce(crate::parser::helper::HtmlDoc) -> Result<T, crate::error::ParseError>,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = client.get_html(&path).await?;
    parse(doc)
        .map(Into::into)
        .map_err(|error| MalError::parse_failed(&path, error.to_string()))
}

/// Like [`fetch_and_parse`], but an upstream 404 yields `empty()` instead of an
/// error.
///
/// Several MAL listings 404 when there is nothing to list, and jikan-php
/// answers those with an empty model rather than a failure.
pub(crate) async fn fetch_and_parse_or_empty<P: ParseModel>(
    client: &dyn MalSource,
    request: impl MalRequest,
    empty: impl FnOnce() -> Value,
) -> Result<Value, MalError> {
    let path = request.path();
    let doc = match client.get_html(&path).await {
        Ok(doc) => doc,
        Err(MalError::BadResponse { status: 404, .. }) => return Ok(empty()),
        Err(error) => return Err(error),
    };
    P::model(doc).map_err(|error| MalError::parse_failed(&path, error.to_string()))
}
