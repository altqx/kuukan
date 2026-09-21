//! The request lifecycle every endpoint runs.
//!
//! Port of `DefaultCachedScraperService` plus the `ResponseJikanCacheFlags`
//! macro. Two storage shapes exist in Jikan:
//!
//! - **entities** keyed by `(kind, mal_id)` (anime, manga, characters, ...),
//!   used by `/anime/{id}`-style endpoints;
//! - **cache documents** keyed by request fingerprint
//!   (`request:<type>:<sha1(uri)>`), used by sub-resource endpoints
//!   (`/anime/{id}/characters`, `/anime/{id}/news`, user pages, ...).
//!
//! Both follow the same lifecycle: read cache -> if missing or expired scrape
//! MAL -> persist -> return. Scrape failures propagate exactly like the PHP
//! exceptions (404 stays 404, upstream errors become `UpstreamException`).
//!
//! # Why a descriptor
//!
//! Each endpoint used to restate five facts at its call site: the request
//! type, the cache category, how the cache key is derived from the URI, the
//! MAL fetch and the resource mapper. Five route modules then re-rolled the
//! same private `load_entity`/`load_cache`/`render` helpers on top.
//!
//! The request type was the expensive one. It was written twice per
//! endpoint — once to key the cache document, once to build the
//! `X-Request-Fingerprint` header — as 34 bare string literals with no single
//! owner and nothing checking the two agreed. A typo in either would have
//! advertised a fingerprint that no cache document was stored under.
//!
//! [`Endpoint`] owns those facts, and [`Cached::render`] reuses the very
//! fingerprint that keyed the lookup, so the header cannot disagree with the
//! cache by construction.

use std::future::Future;

use axum::response::Response;
use kuukan_core::error::ApiError;
use kuukan_core::EntityKind;
use kuukan_mal::error::MalError;
use kuukan_store::StoredEntity;
use serde_json::Value;

use crate::config::CacheCategory;
use crate::error::{mal_error_to_api, ApiErrorResponse};
use crate::render::json_with_cache_flags;
use crate::state::AppState;

/// How an endpoint's cache key is derived from the request URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyRule {
    /// Path and query, as PHP's `getRequestUri()` returns it.
    FullUri,
    /// Path only. Genre lists are media-wide documents: pagination parameters
    /// do not change the document PHP serves them from.
    PathOnly,
}

/// What an endpoint is cached and fingerprinted as.
#[derive(Debug, Clone, Copy)]
pub struct Endpoint {
    request_type: &'static str,
    category: CacheCategory,
    key: KeyRule,
}

impl Endpoint {
    /// An endpoint of `request_type`, with the default TTL, keyed on the full
    /// request URI.
    pub const fn new(request_type: &'static str) -> Self {
        Endpoint {
            request_type,
            category: CacheCategory::Default,
            key: KeyRule::FullUri,
        }
    }

    /// Use a different `per_endpoint_cache_ttl` category.
    #[must_use]
    pub const fn category(mut self, category: CacheCategory) -> Self {
        self.category = category;
        self
    }

    /// Key the cache on the path alone, ignoring query parameters.
    #[must_use]
    pub const fn path_only(mut self) -> Self {
        self.key = KeyRule::PathOnly;
        self
    }

    fn uri(&self, uri: &axum::http::Uri) -> String {
        match self.key {
            KeyRule::FullUri => match uri.query() {
                Some(query) => format!("{}?{}", uri.path(), query),
                None => uri.path().to_string(),
            },
            KeyRule::PathOnly => uri.path().to_string(),
        }
    }

    /// The cache flags for a response with no cached document behind it.
    ///
    /// Repository and search listings are built from the store, so PHP's
    /// `CachedData` is absent; pass `0` for the epoch `Last-Modified` those
    /// endpoints report.
    pub fn flags(&self, state: &AppState, uri: &axum::http::Uri, last_modified: i64) -> CacheFlags {
        CacheFlags {
            fingerprint: fingerprint(self.request_type, &self.uri(uri)),
            ttl: state.config.cache_ttl(self.category),
            last_modified,
        }
    }

    /// Fetch an entity by MAL id, scraping on cache miss or expiry.
    ///
    /// `ItemLookupHandler::handle` + `CachedScraperService::find`.
    pub async fn entity<F, Fut>(
        &self,
        state: &AppState,
        uri: &axum::http::Uri,
        kind: EntityKind,
        mal_id: i64,
        fetch: F,
    ) -> Result<Cached, ApiErrorResponse>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value, MalError>>,
    {
        let ttl = state.config.cache_ttl(self.category);
        let now = kuukan_store::now_unix();

        if let Some(entity) = state
            .store
            .get_entity(kind, mal_id)
            .await
            .map_err(storage_error)?
        {
            // The stored `expires_at` column is authoritative when present
            // (kuukan writes `modifiedAt + endpoint ttl`, so this equals PHP's
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
                return Ok(self.cached(state, uri, entity.payload, entity.modified_at));
            }
        }

        let payload = self.scrape(state, now, fetch).await?;
        let entity = StoredEntity::new(kind, mal_id, payload.clone());
        state
            .store
            .upsert_entity(entity, Some(ttl as i64))
            .await
            .map_err(storage_error)?;
        index_entity(state, kind, &payload);

        Ok(self.cached(state, uri, payload, now))
    }

    /// Fetch a fingerprint-keyed cache document, scraping on miss or expiry.
    ///
    /// `RequestHandlerWithScraperCache::handle` + `findList`.
    pub async fn document<F, Fut>(
        &self,
        state: &AppState,
        uri: &axum::http::Uri,
        fetch: F,
    ) -> Result<Cached, ApiErrorResponse>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value, MalError>>,
    {
        let ttl = state.config.cache_ttl(self.category);
        let now = kuukan_store::now_unix();
        let key = fingerprint(self.request_type, &self.uri(uri));

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
                return Ok(self.cached(state, uri, entry.payload, entry.modified_at));
            }
        }

        let payload = self.scrape(state, now, fetch).await?;
        state
            .store
            .put_cache(&key, payload.clone(), Some(ttl as i64), existed)
            .await
            .map_err(storage_error)?;

        Ok(self.cached(state, uri, payload, now))
    }

    /// Like [`Endpoint::document`], but the cache document is keyed on
    /// `key_uri` instead of the request URI.
    ///
    /// For the endpoints where PHP looks the document up by entity rather than
    /// by request — every `/users/{name}/...` profile variant shares one
    /// document keyed by username — while `X-Request-Fingerprint` still hashes
    /// the request that was actually made. That divergence is deliberate, so
    /// it gets its own method rather than hiding inside the common one.
    pub async fn document_keyed<F, Fut>(
        &self,
        state: &AppState,
        uri: &axum::http::Uri,
        key_uri: &str,
        fetch: F,
    ) -> Result<Cached, ApiErrorResponse>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value, MalError>>,
    {
        let ttl = state.config.cache_ttl(self.category);
        let now = kuukan_store::now_unix();
        let key = fingerprint(self.request_type, key_uri);

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
                return Ok(self.cached(state, uri, entry.payload, entry.modified_at));
            }
        }

        let payload = self.scrape(state, now, fetch).await?;
        state
            .store
            .put_cache(&key, payload.clone(), Some(ttl as i64), existed)
            .await
            .map_err(storage_error)?;

        Ok(self.cached(state, uri, payload, now))
    }

    async fn scrape<F, Fut>(
        &self,
        state: &AppState,
        now: i64,
        fetch: F,
    ) -> Result<Value, ApiErrorResponse>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Value, MalError>>,
    {
        match fetch().await {
            Ok(payload) => Ok(payload),
            Err(err) => {
                let _ = state.store.record_source_result(false, now).await;
                Err(ApiErrorResponse(mal_error_to_api(err)))
            }
        }
    }

    fn cached(
        &self,
        state: &AppState,
        uri: &axum::http::Uri,
        payload: Value,
        modified_at: i64,
    ) -> Cached {
        Cached {
            payload,
            flags: CacheFlags {
                fingerprint: fingerprint(self.request_type, &self.uri(uri)),
                ttl: state.config.cache_ttl(self.category),
                last_modified: modified_at,
            },
        }
    }
}

/// The Jikan cache flags for one response.
///
/// The fingerprint here is the one the lookup used, which is what makes
/// `X-Request-Fingerprint` and the cache key the same string by construction.
#[derive(Debug, Clone)]
pub struct CacheFlags {
    fingerprint: String,
    ttl: u64,
    last_modified: i64,
}

impl CacheFlags {
    /// Render `data` with these flags attached.
    pub fn render(&self, data: Value) -> Response {
        json_with_cache_flags(data, &self.fingerprint, self.last_modified, self.ttl)
    }

    /// The request fingerprint, for the handful of endpoints that need it
    /// before they have a body.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// A cached payload and the flags to render it with.
#[derive(Debug, Clone)]
pub struct Cached {
    pub payload: Value,
    flags: CacheFlags,
}

impl Cached {
    /// Render `data` with this lookup's cache flags.
    pub fn render(&self, data: Value) -> Response {
        self.flags.render(data)
    }

    /// When the cached document was last written.
    pub fn modified_at(&self) -> i64 {
        self.flags.last_modified
    }
}

/// Compute the request fingerprint for a URI.
///
/// Kuukan serves `/v1`, but the fingerprint hashes the *upstream* `/v4` URI so
/// cache documents and `X-Request-Fingerprint` are byte-identical with Jikan
/// (an existing Jikan Mongo cache can be imported directly).
pub fn fingerprint(request_type: &str, uri: &str) -> String {
    kuukan_core::util::jikan_request_fingerprint(request_type, uri)
}

fn is_expired(modified_at: i64, ttl: u64, now: i64) -> bool {
    // CachedData::isExpired(): now > modifiedAt + ttl
    now > modified_at.saturating_add(ttl as i64)
}

/// Feed a freshly scraped entity into the search index, mirroring Scout's
/// synchronous indexing on model save. Failures are logged, not fatal: search
/// availability must not break the entity endpoint.
fn index_entity(state: &AppState, kind: EntityKind, payload: &Value) {
    if !kind.is_searchable() {
        return;
    }
    if let Err(err) = state.pipeline.index_payload(kind, payload) {
        tracing::warn!(
            kind = %kind,
            error = %err,
            "failed to index entity for search"
        );
    }
}

fn storage_error(err: kuukan_store::StoreError) -> ApiErrorResponse {
    ApiErrorResponse(ApiError::Storage {
        error: Some(err.to_string()),
        report_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(value: &str) -> axum::http::Uri {
        value.parse().expect("uri")
    }

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

    #[test]
    fn full_uri_keys_include_the_query() {
        let endpoint = Endpoint::new("anime");
        assert_eq!(
            endpoint.uri(&uri("/v1/anime/1/episodes?page=2")),
            "/v1/anime/1/episodes?page=2"
        );
    }

    /// Genre lists are media-wide documents, so pagination must not split the
    /// cache.
    #[test]
    fn path_only_keys_ignore_the_query() {
        let endpoint = Endpoint::new("genres").path_only();
        assert_eq!(
            endpoint.uri(&uri("/v1/genres/anime?page=2")),
            endpoint.uri(&uri("/v1/genres/anime"))
        );
    }

    /// The whole point: the header advertises the key the document is stored
    /// under.
    #[test]
    fn flags_carry_the_key_the_lookup_uses() {
        let endpoint = Endpoint::new("clubs");
        let request = uri("/v1/clubs/1/members?page=3");
        assert_eq!(
            fingerprint("clubs", &endpoint.uri(&request)),
            fingerprint(endpoint.request_type, &endpoint.uri(&request))
        );
    }
}
