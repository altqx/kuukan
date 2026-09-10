//! Errors for the MAL scraping layer.
//!
//! Mirrors the exception semantics of jikan-php:
//!
//! - `Jikan\Exception\BadResponseException` -> [`MalError::BadResponse`]
//!   (status code is preserved, message is `"<status> on <url>"` exactly like
//!   `Jikan\Goutte\GoutteWrapper::request()`).
//! - `Jikan\Exception\ParserException` -> [`MalError::Parse`] via
//!   [`MalError::parse_failed`], message `"Failed to parse '<path>'"` exactly
//!   like `ParserException::fromRequest()`.
//! - Transport/timeout failures surface as [`MalError::Transport`].
//! - HTML/XPath/CSS infrastructure failures surface as
//!   [`MalError::ParseError`].
//!
//! `MalError::Parse` deliberately keeps the PHP-observable message: the
//! underlying cause passed to [`MalError::parse_failed`] is logged (PHP also
//! keeps it only in the exception chain / GitHub report body, not in the HTTP
//! `error` field).

use thiserror::Error;

/// A failure while building or querying an HTML document.
///
/// Returned by [`crate::parser::helper::HtmlDoc`] and friends; converted into
/// [`MalError::ParseError`] by [`crate::client::MalClient`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// libxml2 refused to build a document from the input.
    #[error("unable to parse HTML: {0}")]
    Html(String),

    /// The parsed document has no root element (empty body).
    #[error("HTML document has no root element")]
    MissingRoot,

    /// An XPath expression failed to evaluate (syntax error or nodeset limit).
    #[error("invalid XPath expression: {0}")]
    InvalidXPath(String),

    /// A CSS selector could not be translated to XPath.
    #[error("invalid CSS selector: {0}")]
    InvalidSelector(String),
}

/// Errors returned by [`crate::client::MalClient`].
#[derive(Debug, Error)]
pub enum MalError {
    /// MAL answered with an HTTP error status (`>= 400`).
    ///
    /// Mirrors `Jikan\Exception\BadResponseException`, whose message is
    /// `"<status> on <uri>"` (`GoutteWrapper::request()`), and preserves the
    /// status code so `App\Exceptions\Handler` can classify it.
    #[error("{status} on {url}")]
    BadResponse { status: u16, url: String },

    /// Network/transport failure (connect, TLS, timeout, body read).
    #[error("{0}")]
    Transport(String),

    /// Request timed out. Distinct variant so the API can render PHP's
    /// `UpstreamException` timeout body (`timed out (N seconds)`).
    #[error("{message}")]
    Timeout { seconds: u64, message: String },

    /// Parser failure. Use [`MalError::parse_failed`] to build the PHP
    /// `ParserException` message.
    #[error("{0}")]
    Parse(String),

    /// HTML/XPath/CSS parsing infrastructure failure.
    #[error(transparent)]
    ParseError(#[from] ParseError),

    /// The response body was not valid JSON (`get_json` only).
    #[error("invalid JSON response: {0}")]
    Json(String),
}

impl MalError {
    /// Build a parser failure with the exact message of
    /// `Jikan\Exception\ParserException::fromRequest()`:
    /// `Failed to parse '<source_file>'`.
    ///
    /// `message` (the underlying cause, e.g. a panic/DOM error) is not part of
    /// the PHP exception message either — it only reaches logs and report
    /// bodies — so it is logged here and omitted from the HTTP-visible text.
    pub fn parse_failed(source_file: impl AsRef<str>, message: impl AsRef<str>) -> Self {
        let source_file = source_file.as_ref();
        let message = message.as_ref();
        tracing::error!(
            source_file = %source_file,
            error = %message,
            "MAL parser failed"
        );
        MalError::Parse(format!("Failed to parse '{source_file}'"))
    }

    /// The upstream HTTP status, when this error came from MAL itself.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            MalError::BadResponse { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Whether the error is a MAL 404 (the only status never retried).
    pub fn is_not_found(&self) -> bool {
        matches!(self, MalError::BadResponse { status: 404, .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_response_message_matches_goutte() {
        let err = MalError::BadResponse {
            status: 404,
            url: "https://myanimelist.net/anime/1".into(),
        };
        assert_eq!(err.to_string(), "404 on https://myanimelist.net/anime/1");
        assert_eq!(err.status_code(), Some(404));
        assert!(err.is_not_found());
    }

    #[test]
    fn parse_failed_matches_parser_exception() {
        let err = MalError::parse_failed("https://myanimelist.net/anime/1", "index out of range");
        assert_eq!(
            err.to_string(),
            "Failed to parse 'https://myanimelist.net/anime/1'"
        );
    }

    #[test]
    fn parse_error_converts() {
        let err: MalError = ParseError::InvalidXPath("//[".into()).into();
        assert!(matches!(err, MalError::ParseError(_)));
    }
}
