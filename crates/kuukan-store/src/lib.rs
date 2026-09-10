//! Kuukan storage layer.
//!
//! A single SQLite database replaces the MongoDB + Redis pair of jikan-rest.
//! The crate owns four storage concerns:
//!
//! * **Entities** ([`StoredEntity`]): complete API-shaped JSON documents keyed
//!   by [`EntityKind`] and MAL id, written by the indexers and read by the API
//!   resources. Timestamps are unix seconds; `expires_at` is informational and
//!   `NULL` means "never expires".
//! * **Cache** ([`CacheEntry`]): scraper results keyed by request fingerprint,
//!   with the `createdAt`/`modifiedAt`/`request_hash` meta handling of
//!   `DefaultCachedScraperService`.
//! * **Source health** ([`Health`]): MAL heartbeat records and failover state,
//!   ported from `SourceHeartbeatProvider`/`SourceHeartbeatListener`.
//! * **Auxiliary state**: search metrics and indexer cursors
//!   ([`SearchMetric`], [`Store::get_indexer_cursor`]).
//!
//! # Opening a store
//!
//! ```no_run
//! use kuukan_store::{Store, StoreConfig};
//!
//! # async fn example() -> Result<(), kuukan_store::StoreError> {
//! let store = Store::open(StoreConfig::from_env()).await?;
//! # Ok(())
//! # }
//! ```
//!
//! [`Store::open`] creates the database file and parent directories, enables
//! WAL journalling, a busy timeout and foreign keys on every connection, and
//! runs the embedded migrations.
//!
//! # Time and TTLs
//!
//! All timestamps are unix seconds (`i64`). The store never guesses a TTL:
//! callers pass `ttl_secs` (from jikan's `config/jikan.php` and
//! `CACHE_*`/`CACHE_DEFAULT_EXPIRE` variables) and the store records
//! `expires_at = modified_at + ttl`. Expired rows are still returned; the
//! caller decides whether to re-scrape or serve stale data.
//!
//! # Errors
//!
//! Every fallible operation returns [`StoreError`]; API paths never panic.

pub mod cache;
pub mod config;
pub mod db;
pub mod entities;
pub mod health;
pub mod import;
pub mod migrations;

pub use cache::CacheEntry;
pub use config::StoreConfig;
pub use db::{now_unix, SearchMetric, Store, StoreError};
pub use entities::{EntityKind, StoredEntity};
pub use health::{Health, HealthRecord, HealthThresholds};
pub use import::ImportReport;
