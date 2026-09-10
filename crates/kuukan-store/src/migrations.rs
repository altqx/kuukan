//! Embedded SQLite schema migrations.
//!
//! Migration scripts live in `crates/kuukan-store/migrations/` and are embedded
//! into the binary by [`sqlx::migrate!`], so no migration tooling is needed at
//! runtime. [`crate::Store::open`] runs all pending migrations automatically;
//! [`run`] is exposed for callers that manage their own pool.

use sqlx::SqlitePool;

use crate::db::StoreError;

/// Embedded migration set (`./migrations` relative to this crate's manifest).
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Run all pending migrations against `pool`.
pub async fn run(pool: &SqlitePool) -> Result<(), StoreError> {
    MIGRATOR.run(pool).await?;
    Ok(())
}
