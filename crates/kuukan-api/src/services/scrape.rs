//! Cached-scrape orchestration, port of
//! `app/Services/DefaultCachedScraperService.php`.
//!
//! Two storage shapes exist in Jikan:
//!
//! - **entities** keyed by `(kind, mal_id)` (anime, manga, characters, ...),
//!   used by `/anime/{id}`-style endpoints;
//! - **cache documents** keyed by request fingerprint
//!   (`request:<type>:<sha1(uri)>`), used by sub-resource endpoints
//!   (`/anime/{id}/characters`, `/anime/{id}/news`, user pages, ...).
//!
//! Both follow the same lifecycle: read cache -> if missing/expired scrape MAL
//! -> persist -> return. Scrape failures propagate exactly like the PHP
//! exceptions (404 stays 404, upstream errors become `UpstreamException`).

use std::future::Future;

use kuukan_core::error::ApiError;
use kuukan_mal::error::MalError;
use kuukan_store::{EntityKind, StoredEntity};
use serde_json::Value;

use crate::error::mal_error_to_api;
use crate::state::AppState;

/// Result of a cache lookup: the payload and its `modifiedAt` timestamp.
#[derive(Debug, Clone)]
pub struct CachedPayload {
    pub payload: Value,
    pub modified_at: i64,
}

/// Compute the request fingerprint for a URI (helper for handlers).
///
/// Kuukan serves `/v1`, but the fingerprint hashes the *upstream* `/v4` URI so
/// cache documents and `X-Request-Fingerprint` are byte-identical with Jikan
/// (an existing Jikan Mongo cache can be imported directly).
pub fn fingerprint(request_type: &str, uri: &str) -> String {
    kuukan_core::util::jikan_request_fingerprint(request_type, uri)
}

/// Full request URI (path + query) as PHP's `getRequestUri()` returns it.
pub fn request_uri(uri: &axum::http::Uri) -> String {
    match uri.query() {
        Some(query) => format!("{}?{}", uri.path(), query),
        None => uri.path().to_string(),
    }
}

fn is_expired(modified_at: i64, ttl: u64, now: i64) -> bool {
    // CachedData::isExpired(): now > modifiedAt + ttl
    now > modified_at.saturating_add(ttl as i64)
}

/// Fetch an entity by MAL id, scraping on cache miss/expiry.
pub async fn entity_or_scrape<F, Fut>(
    state: &AppState,
    kind: EntityKind,
    mal_id: i64,
    ttl: u64,
    fetch: F,
) -> Result<CachedPayload, ApiError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Value, MalError>>,
{
    let now = kuukan_store::now_unix();

    if let Some(entity) = state
        .store
        .get_entity(kind, mal_id)
        .await
        .map_err(storage_error)?
    {
        // The stored `expires_at` column is authoritative when present (kuukan
        // writes `modifiedAt + endpoint ttl`, so this equals PHP's
        // CachedData::isExpired()); imported rows with NULL fall back to
        // `modifiedAt + ttl`.
        let stale = entity
            .expires_at
            .map(|expires_at| now > expires_at)
            .unwrap_or_else(|| is_expired(entity.modified_at, ttl, now));
        if !stale {
            state
                .store
                .record_source_result(true, now)
                .await
                .map_err(storage_error)?;
            return Ok(CachedPayload {
                payload: entity.payload,
                modified_at: entity.modified_at,
            });
        }
    }

    let payload = match fetch().await {
        Ok(payload) => payload,
        Err(err) => {
            let _ = state.store.record_source_result(false, now).await;
            return Err(mal_error_to_api(err));
        }
    };

    let entity = StoredEntity::new(kind, mal_id, payload.clone());
    state
        .store
        .upsert_entity(entity, Some(ttl as i64))
        .await
        .map_err(storage_error)?;
    index_entity(state, kind, &payload);

    Ok(CachedPayload {
        payload,
        modified_at: now,
    })
}

/// Feed a freshly scraped entity into the search index, mirroring Scout's
/// synchronous indexing on model save. Failures are logged, not fatal: search
/// availability must not break the entity endpoint.
fn index_entity(state: &AppState, kind: EntityKind, payload: &Value) {
    let Some(search_kind) = search_kind(kind) else {
        return;
    };
    if let Err(err) = state.pipeline.index_payload(search_kind, payload) {
        tracing::warn!(
            kind = ?kind,
            error = %err,
            "failed to index entity for search"
        );
    }
}

fn search_kind(kind: EntityKind) -> Option<kuukan_search::EntityKind> {
    use kuukan_search::EntityKind as SearchKind;
    match kind {
        EntityKind::Anime => Some(SearchKind::Anime),
        EntityKind::Manga => Some(SearchKind::Manga),
        EntityKind::Character => Some(SearchKind::Character),
        EntityKind::Person => Some(SearchKind::Person),
        EntityKind::User => Some(SearchKind::User),
        EntityKind::Club => Some(SearchKind::Club),
        EntityKind::Producer => Some(SearchKind::Producer),
        EntityKind::Magazine => Some(SearchKind::Magazine),
        EntityKind::GenreAnime | EntityKind::GenreManga | EntityKind::Episode => None,
    }
}

/// Fetch a fingerprint-keyed cache document, scraping on miss/expiry.
pub async fn cache_or_scrape<F, Fut>(
    state: &AppState,
    request_type: &str,
    uri: &str,
    ttl: u64,
    fetch: F,
) -> Result<CachedPayload, ApiError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Value, MalError>>,
{
    let now = kuukan_store::now_unix();
    let key = fingerprint(request_type, uri);

    let mut existed = false;
    if let Some(entry) = state.store.get_cache(&key).await.map_err(storage_error)? {
        existed = true;
        let stale = entry
            .expires_at
            .map(|expires_at| now > expires_at)
            .unwrap_or_else(|| is_expired(entry.modified_at, ttl, now));
        if !stale {
            state
                .store
                .record_source_result(true, now)
                .await
                .map_err(storage_error)?;
            return Ok(CachedPayload {
                payload: entry.payload,
                modified_at: entry.modified_at,
            });
        }
    }

    let payload = match fetch().await {
        Ok(payload) => payload,
        Err(err) => {
            let _ = state.store.record_source_result(false, now).await;
            return Err(mal_error_to_api(err));
        }
    };

    state
        .store
        .put_cache(&key, payload.clone(), Some(ttl as i64), existed)
        .await
        .map_err(storage_error)?;

    Ok(CachedPayload {
        payload,
        modified_at: now,
    })
}

fn storage_error(err: kuukan_store::StoreError) -> ApiError {
    ApiError::Storage {
        error: Some(err.to_string()),
        report_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_matches_php() {
        assert!(!is_expired(1000, 3600, 1500));
        assert!(!is_expired(1000, 3600, 4600));
        assert!(is_expired(1000, 3600, 4601));
    }

    #[test]
    fn fingerprint_has_php_format() {
        // `/v1` is normalized to the upstream `/v4` URI so the fingerprint
        // matches Jikan's request_hash exactly.
        assert_eq!(
            fingerprint("anime", "/v1/anime/1"),
            kuukan_core::util::request_fingerprint("anime", "/v4/anime/1")
        );
        assert_eq!(
            fingerprint("anime", "/v1/anime/1/characters?page=2"),
            kuukan_core::util::request_fingerprint("anime", "/v4/anime/1/characters?page=2")
        );
    }
}
