//! SQLite connection management and the [`Store`] handle.
//!
//! [`Store`] owns a [`SqlitePool`] and is the entry point for every storage
//! operation in Kuukan. Opening a store:
//!
//! 1. creates the database file (and its parent directories) if missing,
//! 2. enables WAL journalling, a busy timeout and foreign keys on every
//!    connection,
//! 3. runs all embedded migrations (see [`crate::migrations`]).
//!
//! Timestamps are unix seconds (`i64`). `expires_at = NULL` always means
//! "never expires"; TTL selection is the caller's job (jikan-rest reads the
//! per-endpoint values from `config/jikan.php` and `CacheOptions`).
//!
//! This module also hosts the small tables without a dedicated module:
//! `search_metrics` (search analytics) and `indexer_state` (resumable indexer
//! cursors).

use std::time::Duration;

use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::config::StoreConfig;

/// Errors returned by the storage layer.
///
/// API paths must never panic; every fallible operation returns one of these.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// Underlying SQLite/sqlx failure.
    #[error("sqlite error: {0}")]
    Database(#[from] sqlx::Error),
    /// Migration failure while opening or migrating a store.
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// Stored JSON could not be parsed or serialized.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Filesystem error (database path or import file).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// A `kind` string does not map to [`crate::EntityKind`].
    #[error("unknown entity kind: {0}")]
    InvalidKind(String),
    /// A document did not have the expected shape.
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
}

/// Current wall-clock time as unix seconds.
///
/// Clock errors (pre-epoch clocks) collapse to 0 rather than panicking.
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// A handle to the Kuukan SQLite database.
///
/// Cloning a `Store` clones the pool handle; all clones share the same
/// connections.
#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Open (or create) the database described by `config` and run migrations.
    pub async fn open(config: StoreConfig) -> Result<Self, StoreError> {
        if let Some(parent) = config.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let options = SqliteConnectOptions::new()
            .filename(&config.path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections.max(1))
            .connect_with(options)
            .await?;

        Self::from_pool(pool).await
    }

    /// Open a private in-memory database, used by tests and short-lived tools.
    ///
    /// The pool is pinned to a single connection: an in-memory SQLite database
    /// only exists for as long as its connection does.
    pub async fn open_in_memory() -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .journal_mode(SqliteJournalMode::Memory)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(options)
            .await?;

        Self::from_pool(pool).await
    }

    async fn from_pool(pool: SqlitePool) -> Result<Self, StoreError> {
        let store = Self { pool };
        store.migrate().await?;
        Ok(store)
    }

    /// Run all pending migrations. Called automatically by [`Store::open`].
    pub async fn migrate(&self) -> Result<(), StoreError> {
        crate::migrations::run(&self.pool).await
    }

    /// Access the underlying pool for ad-hoc queries.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the pool, waiting for in-flight queries to finish.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// A logged search query (`search_metrics` table).
///
/// jikan-rest's `DefaultSearchAnalyticsService` keeps one row per search term
/// with request/hit counters; Kuukan stores an append-only log and leaves
/// aggregation to the caller (the `insights` endpoints).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SearchMetric {
    /// Autoincrement row id.
    pub id: i64,
    /// The search term (`q` query parameter).
    pub query: String,
    /// Free-form caller metadata (result count, index name, ...).
    pub meta: Option<Value>,
    /// Unix seconds when the search was recorded.
    pub created_at: i64,
}

#[derive(sqlx::FromRow)]
struct SearchMetricRow {
    id: i64,
    query: String,
    meta: Option<String>,
    created_at: i64,
}

impl TryFrom<SearchMetricRow> for SearchMetric {
    type Error = StoreError;

    fn try_from(row: SearchMetricRow) -> Result<Self, StoreError> {
        let meta = match row.meta {
            Some(meta) => Some(serde_json::from_str(&meta)?),
            None => None,
        };
        Ok(SearchMetric {
            id: row.id,
            query: row.query,
            meta,
            created_at: row.created_at,
        })
    }
}

impl Store {
    /// Append a search query to the analytics log.
    pub async fn record_search(&self, query: &str, meta: Option<&Value>) -> Result<(), StoreError> {
        let meta = match meta {
            Some(meta) => Some(serde_json::to_string(meta)?),
            None => None,
        };
        sqlx::query("INSERT INTO search_metrics (query, meta, created_at) VALUES (?, ?, ?)")
            .bind(query)
            .bind(meta)
            .bind(now_unix())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return the `limit` most recent search queries, newest first.
    pub async fn recent_searches(&self, limit: i64) -> Result<Vec<SearchMetric>, StoreError> {
        let limit = limit.max(0);
        let rows = sqlx::query_as::<_, SearchMetricRow>(
            "SELECT id, query, meta, created_at FROM search_metrics \
             ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(SearchMetric::try_from).collect()
    }

    /// Read a resumable indexer cursor, or `None` when unset.
    ///
    /// A row with a SQL `NULL` cursor also yields `None`.
    pub async fn get_indexer_cursor(&self, name: &str) -> Result<Option<String>, StoreError> {
        let cursor = sqlx::query_scalar::<_, Option<String>>(
            "SELECT cursor FROM indexer_state WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(cursor.flatten())
    }

    /// Store (or overwrite) a resumable indexer cursor.
    pub async fn set_indexer_cursor(&self, name: &str, cursor: &str) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO indexer_state (name, cursor, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(name) DO UPDATE SET cursor = excluded.cursor, \
             updated_at = excluded.updated_at",
        )
        .bind(name)
        .bind(cursor)
        .bind(now_unix())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityKind;

    #[tokio::test]
    async fn file_database_persists_across_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("kuukan.db");

        let store = Store::open(StoreConfig::new(&path)).await.expect("open");
        store
            .upsert_entity(
                crate::StoredEntity::new(EntityKind::Anime, 1, serde_json::json!({"mal_id": 1})),
                Some(60),
            )
            .await
            .expect("upsert");
        store.close().await;

        let reopened = Store::open(StoreConfig::new(&path)).await.expect("reopen");
        let entity = reopened
            .get_entity(EntityKind::Anime, 1)
            .await
            .expect("get")
            .expect("entity present");
        assert_eq!(entity.payload, serde_json::json!({"mal_id": 1}));
    }

    #[tokio::test]
    async fn open_applies_pragmas() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("kuukan.db");
        let store = Store::open(StoreConfig::new(&path)).await.expect("open");

        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(store.pool())
            .await
            .expect("journal_mode");
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(store.pool())
            .await
            .expect("foreign_keys");
        assert_eq!(foreign_keys, 1);

        let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(store.pool())
            .await
            .expect("busy_timeout");
        assert_eq!(busy_timeout, 5000);
    }

    #[tokio::test]
    async fn search_metrics_are_recorded_newest_first() {
        let store = Store::open_in_memory().await.expect("open");
        store.record_search("naruto", None).await.expect("record");
        store
            .record_search("bleach", Some(&serde_json::json!({"hits": 3})))
            .await
            .expect("record");

        let recent = store.recent_searches(10).await.expect("recent");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].query, "bleach");
        assert_eq!(recent[0].meta, Some(serde_json::json!({"hits": 3})));
        assert_eq!(recent[1].query, "naruto");
        assert_eq!(recent[1].meta, None);
        assert!(recent[0].created_at >= recent[1].created_at);
    }

    #[tokio::test]
    async fn indexer_cursors_round_trip() {
        let store = Store::open_in_memory().await.expect("open");
        assert_eq!(store.get_indexer_cursor("anime").await.expect("get"), None);

        store.set_indexer_cursor("anime", "42").await.expect("set");
        assert_eq!(
            store.get_indexer_cursor("anime").await.expect("get"),
            Some("42".to_string())
        );

        store.set_indexer_cursor("anime", "43").await.expect("set");
        assert_eq!(
            store.get_indexer_cursor("anime").await.expect("get"),
            Some("43".to_string())
        );
    }
}
