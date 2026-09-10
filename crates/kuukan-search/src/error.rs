//! Error types for the search crate.

use thiserror::Error;

/// Errors produced by `kuukan-search`.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Tantivy failed to open/create/read an index.
    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    /// (De)serializing the stored payload failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem error while creating/removing index directories.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The entity kind name has no known schema.
    #[error("unknown entity kind: {0}")]
    UnknownKind(String),

    /// A payload can not be turned into an index document.
    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    /// The Tantivy query parser could not build a query.
    #[error("invalid search query: {0}")]
    InvalidQuery(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, SearchError>;
