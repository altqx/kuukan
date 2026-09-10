//! Full-text search and filtering for Kuukan, on top of Tantivy.
//!
//! This crate replaces Typesense for the Jikan REST v4 surface. It is
//! intentionally independent from `kuukan-store`: the indexer (and the HTTP
//! layer) hand it JMS-shaped `serde_json::Value` payloads, it extracts the
//! searchable/filterable fields, and returns the stored payloads back.
//!
//! Layout:
//! - [`schema`]: one Tantivy schema per [`schema::EntityKind`], plus payload
//!   extraction (what each Jikan model exposes to search).
//! - [`index`]: [`index::SearchIndex`], the directory-backed set of
//!   sub-indexes with writer/reader management.
//! - [`query`]: [`query::SearchParams`] / [`query::SearchResult`], the port of
//!   `TypeSenseScoutSearchService` + `MediaFilters` + `FilterQueryString`.
//! - [`pipeline`]: [`pipeline::IndexPipeline`] batching helper.
//!
//! TODO(coordinator): `EntityKind` is defined here because `kuukan-core` is
//! still empty. If a workspace-wide entity kind is introduced, re-export or
//! move this enum there.

pub mod error;
pub mod index;
pub mod pipeline;
pub mod query;
pub mod schema;

pub use error::{Result, SearchError};
pub use index::SearchIndex;
pub use pipeline::IndexPipeline;
pub use query::{SearchParams, SearchResult, SortDirection};
pub use schema::EntityKind;
