//! Cache entry storage (`cache_entries` table).
//!
//! Replaces the Redis cache of jikan-rest. The row key is the request
//! fingerprint (`X-Request-Fingerprint` values such as
//! `request:anime:<sha1>`); the value is the serialized scraper result.
//!
//! Timestamp/meta handling mirrors
//! `DefaultCachedScraperService::prepareScraperResponse`:
//!
//! * on insert, the payload gets `createdAt` and `request_hash` (plus
//!   `modifiedAt`);
//! * on update, only `modifiedAt` is overwritten; the stored `createdAt` and
//!   `request_hash` are carried over when the new payload does not include
//!   them (MongoDB `$set` semantics);
//! * `expires_at` is stored as `modified_at + ttl_secs`; a `NULL` expiry
//!   never expires.
//!
//! Callers choose the TTL (`CacheOptions` in jikan-rest) and decide when to
//! re-scrape; expired rows are still returned by [`Store::get_cache`].

use serde_json::{json, Value};

use crate::db::{now_unix, Store, StoreError};

/// A cached scraper result.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    /// Request fingerprint (primary key).
    pub fingerprint: String,
    /// Serialized scraper result, including the `createdAt`/`modifiedAt`/
    /// `request_hash` meta fields.
    pub payload: Value,
    /// Unix seconds of the first insert (kept across updates).
    pub created_at: i64,
    /// Unix seconds of the last write.
    pub modified_at: i64,
    /// Unix seconds after which the entry is stale, or `None` for never.
    pub expires_at: Option<i64>,
}

impl CacheEntry {
    /// Whether the entry is stale at `now` (mirrors `CachedData::isExpired`:
    /// `now > expires_at`). `None` never expires.
    pub fn is_expired(&self, now: i64) -> bool {
        matches!(self.expires_at, Some(expires_at) if now > expires_at)
    }
}

#[derive(sqlx::FromRow)]
struct CacheRow {
    fingerprint: String,
    payload: String,
    created_at: i64,
    modified_at: i64,
    expires_at: Option<i64>,
}

impl TryFrom<CacheRow> for CacheEntry {
    type Error = StoreError;

    fn try_from(row: CacheRow) -> Result<Self, StoreError> {
        Ok(CacheEntry {
            fingerprint: row.fingerprint,
            payload: serde_json::from_str(&row.payload)?,
            created_at: row.created_at,
            modified_at: row.modified_at,
            expires_at: row.expires_at,
        })
    }
}

/// Escape `%`, `_` and `\` so a user-supplied prefix is matched literally in a
/// `LIKE ... ESCAPE '\'` query.
fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

impl Store {
    /// Fetch a cache entry by fingerprint, including expired ones.
    pub async fn get_cache(&self, fingerprint: &str) -> Result<Option<CacheEntry>, StoreError> {
        let row = sqlx::query_as::<_, CacheRow>(
            "SELECT fingerprint, payload, created_at, modified_at, expires_at \
             FROM cache_entries WHERE fingerprint = ?",
        )
        .bind(fingerprint)
        .fetch_optional(self.pool())
        .await?;
        row.map(CacheEntry::try_from).transpose()
    }

    /// Insert or refresh a cache entry.
    ///
    /// `existing` tells the store whether the caller found an existing entry
    /// (jikan-rest's `CachedData::isEmpty()`): on a fresh insert the payload
    /// receives `createdAt`, `request_hash` and `modifiedAt`; on a refresh only
    /// `modifiedAt` is bumped. `expires_at` becomes `now + ttl_secs` when a TTL
    /// is given, otherwise `NULL`.
    pub async fn put_cache(
        &self,
        fingerprint: &str,
        payload: Value,
        ttl_secs: Option<i64>,
        existing: bool,
    ) -> Result<(), StoreError> {
        self.put_cache_at(fingerprint, payload, ttl_secs, existing, now_unix())
            .await
    }

    /// Timestamp-injecting variant of [`Store::put_cache`], for deterministic
    /// tests.
    pub(crate) async fn put_cache_at(
        &self,
        fingerprint: &str,
        payload: Value,
        ttl_secs: Option<i64>,
        existing: bool,
        now: i64,
    ) -> Result<(), StoreError> {
        let mut payload = payload;

        // On refresh, the stored document keeps its original createdAt and
        // request_hash unless the fresh payload provides them (MongoDB `$set`).
        if existing {
            let previous = self.get_cache(fingerprint).await?;
            if let Some(Value::Object(old_map)) = previous.as_ref().map(|entry| &entry.payload) {
                if let Value::Object(new_map) = &mut payload {
                    for key in ["createdAt", "request_hash"] {
                        if !new_map.contains_key(key) {
                            if let Some(value) = old_map.get(key) {
                                new_map.insert(key.to_string(), value.clone());
                            }
                        }
                    }
                }
            }
        }

        if let Value::Object(map) = &mut payload {
            if !existing {
                map.insert("createdAt".to_string(), json!(now));
                map.insert("request_hash".to_string(), json!(fingerprint));
            }
            map.insert("modifiedAt".to_string(), json!(now));
        }

        let payload = serde_json::to_string(&payload)?;
        let expires_at = ttl_secs.map(|ttl| now.saturating_add(ttl));
        sqlx::query(
            "INSERT INTO cache_entries (fingerprint, payload, created_at, modified_at, expires_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(fingerprint) DO UPDATE SET \
                 payload = excluded.payload, \
                 modified_at = excluded.modified_at, \
                 expires_at = excluded.expires_at",
        )
        .bind(fingerprint)
        .bind(payload)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Delete one cache entry. Returns `true` when a row was removed.
    pub async fn delete_cache(&self, fingerprint: &str) -> Result<bool, StoreError> {
        let result = sqlx::query("DELETE FROM cache_entries WHERE fingerprint = ?")
            .bind(fingerprint)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete every entry whose fingerprint starts with `prefix`; returns the
    /// number of removed rows.
    ///
    /// Mirrors `php artisan cache:remove` for prefixed key families (e.g.
    /// `request:anime:` or the `ttl:`-prefixed keys of jikan-rest).
    pub async fn delete_cache_by_prefix(&self, prefix: &str) -> Result<u64, StoreError> {
        let pattern = format!("{}%", escape_like(prefix));
        let result = sqlx::query("DELETE FROM cache_entries WHERE fingerprint LIKE ? ESCAPE '\\'")
            .bind(pattern)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete cache entries whose `expires_at` is in the past; returns the
    /// number of removed rows. Entries without a TTL are never purged.
    pub async fn purge_expired(&self) -> Result<u64, StoreError> {
        self.purge_expired_at(now_unix()).await
    }

    /// Timestamp-injecting variant of [`Store::purge_expired`], for
    /// deterministic tests.
    pub(crate) async fn purge_expired_at(&self, now: i64) -> Result<u64, StoreError> {
        let result = sqlx::query(
            "DELETE FROM cache_entries WHERE expires_at IS NOT NULL AND expires_at < ?",
        )
        .bind(now)
        .execute(self.pool())
        .await?;
        Ok(result.rows_affected())
    }

    /// Number of cached entries.
    pub async fn cache_count(&self) -> Result<i64, StoreError> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cache_entries")
            .fetch_one(self.pool())
            .await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_payload(revision: i64) -> Value {
        json!({"data": [{"revision": revision}]})
    }

    #[tokio::test]
    async fn cache_round_trip_and_meta_handling() {
        let store = Store::open_in_memory().await.expect("open");
        let fingerprint = "request:anime:1";

        store
            .put_cache_at(fingerprint, object_payload(1), Some(120), false, 1_000)
            .await
            .expect("insert");

        let inserted = store
            .get_cache(fingerprint)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(inserted.created_at, 1_000);
        assert_eq!(inserted.modified_at, 1_000);
        assert_eq!(inserted.expires_at, Some(1_120));
        assert_eq!(inserted.payload["createdAt"], 1_000);
        assert_eq!(inserted.payload["modifiedAt"], 1_000);
        assert_eq!(inserted.payload["request_hash"], fingerprint);
        assert_eq!(inserted.payload["data"][0]["revision"], 1);
        assert!(!inserted.is_expired(1_120));
        assert!(inserted.is_expired(1_121));

        store
            .put_cache_at(fingerprint, object_payload(2), Some(60), true, 2_000)
            .await
            .expect("update");

        let updated = store
            .get_cache(fingerprint)
            .await
            .expect("get")
            .expect("present");
        assert_eq!(updated.created_at, 1_000, "created_at survives updates");
        assert_eq!(updated.modified_at, 2_000);
        assert_eq!(updated.expires_at, Some(2_060));
        assert_eq!(
            updated.payload["createdAt"], 1_000,
            "payload createdAt is carried over on refresh"
        );
        assert_eq!(updated.payload["request_hash"], fingerprint);
        assert_eq!(updated.payload["modifiedAt"], 2_000);
        assert_eq!(updated.payload["data"][0]["revision"], 2);
    }

    #[tokio::test]
    async fn cache_prefix_delete_is_literal() {
        let store = Store::open_in_memory().await.expect("open");
        let fingerprints = [
            "request:anime:1",
            "request:anime:12",
            "request:anime:12x",
            "request:manga:1",
            "ttl:request:anime:1",
        ];
        for fingerprint in fingerprints {
            store
                .put_cache(fingerprint, object_payload(1), Some(60), false)
                .await
                .expect("put");
        }
        assert_eq!(store.cache_count().await.expect("count"), 5);

        // `1` is a prefix of `1`, `12` and `12x`.
        let removed = store
            .delete_cache_by_prefix("request:anime:1")
            .await
            .expect("delete prefix");
        assert_eq!(removed, 3);
        assert_eq!(store.cache_count().await.expect("count"), 2);

        assert!(store.delete_cache("request:manga:1").await.expect("delete"));
        assert!(!store.delete_cache("request:manga:1").await.expect("delete"));

        // `%` and `_` are matched literally, not as LIKE wildcards.
        store
            .put_cache("a_b", object_payload(1), None, false)
            .await
            .expect("put");
        store
            .put_cache("axb", object_payload(1), None, false)
            .await
            .expect("put");
        assert_eq!(
            store
                .delete_cache_by_prefix("a_b")
                .await
                .expect("delete prefix"),
            1
        );
    }

    #[tokio::test]
    async fn purge_expired_removes_only_timed_out_entries() {
        let store = Store::open_in_memory().await.expect("open");
        store
            .put_cache_at("expired", object_payload(1), Some(-1), false, 1_000)
            .await
            .expect("put");
        store
            .put_cache_at("alive", object_payload(1), Some(1_000), false, 1_000)
            .await
            .expect("put");
        store
            .put_cache_at("forever", object_payload(1), None, false, 1_000)
            .await
            .expect("put");

        let removed = store.purge_expired_at(1_000).await.expect("purge");
        assert_eq!(removed, 1);
        assert_eq!(store.cache_count().await.expect("count"), 2);
        assert!(store.get_cache("alive").await.expect("get").is_some());
        assert!(store.get_cache("forever").await.expect("get").is_some());
    }
}
