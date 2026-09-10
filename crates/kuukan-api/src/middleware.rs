//! HTTP middleware: micro-caching, optional rate limiting and CORS, mirroring
//! `app/Http/Middleware/{MicroCaching,EndpointCacheTtlMiddleware,CorsMiddleware}.php`.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use kuukan_core::error::ApiError;
use kuukan_core::util::request_fingerprint;
use moka::future::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::ApiErrorResponse;

/// A cached response body (status + headers + bytes).
#[derive(Clone)]
pub struct CachedResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

/// Shared micro-cache state.
#[derive(Clone)]
pub struct MicroCache {
    pub enabled: bool,
    pub cache: Cache<String, CachedResponse>,
}

impl MicroCache {
    pub fn new(enabled: bool, ttl_secs: u64) -> Self {
        MicroCache {
            enabled,
            cache: Cache::builder()
                .time_to_live(Duration::from_secs(ttl_secs.max(1)))
                .max_capacity(10_000)
                .build(),
        }
    }
}

/// Endpoints that must never be micro-cached (PHP `MicroCaching::NO_CACHING`).
fn is_no_caching(path: &str) -> bool {
    path.starts_with("/v1/random") || path.starts_with("/v1/insights")
}

/// Extract the request "type" (anime, manga, users, ...).
/// Kuukan serves everything under `/v1`, so the type is the second segment.
pub fn request_type(path: &str) -> String {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.is_empty() || segments[0].is_empty() {
        return String::new();
    }
    if segments[0] == "v1" {
        segments.get(1).copied().unwrap_or("").to_string()
    } else {
        segments[0].to_string()
    }
}

/// Full request URI (path + query) the way PHP's `getRequestUri()` returns it.
fn request_uri_parts(req: &Request) -> String {
    match req.uri().query() {
        Some(q) => format!("{}?{}", req.uri().path(), q),
        None => req.uri().path().to_string(),
    }
}

/// `sha1(resolveRequestFingerprint(request))` — the micro-cache key.
fn micro_cache_key(req: &Request) -> String {
    let uri = request_uri_parts(req);
    let ty = request_type(req.uri().path());
    kuukan_core::util::sha1_hex(&request_fingerprint(&ty, &uri))
}

/// `MicroCaching` middleware: return a cached JSON body when fresh, otherwise
/// capture the handler response.
pub async fn microcache_middleware(
    State(micro): State<MicroCache>,
    req: Request,
    next: Next,
) -> Response {
    if !micro.enabled || req.method() != Method::GET || is_no_caching(req.uri().path()) {
        return next.run(req).await;
    }

    // Allow bypass of caching if APP_KEY supplied in the auth header.
    if let (Ok(app_key), Some(auth)) = (std::env::var("APP_KEY"), req.headers().get("auth")) {
        if !app_key.is_empty() && auth.to_str().ok() == Some(app_key.as_str()) {
            return next.run(req).await;
        }
    }

    let key = micro_cache_key(&req);
    if let Some(cached) = micro.cache.get(&key).await {
        return rebuild(cached);
    }

    let response = next.run(req).await;
    if response.status() == StatusCode::OK {
        let (parts, body) = response.into_parts();
        if let Ok(bytes) = axum::body::to_bytes(body, 32 * 1024 * 1024).await {
            let cached = CachedResponse {
                status: parts.status,
                headers: parts.headers.clone(),
                body: bytes.clone(),
            };
            micro.cache.insert(key, cached).await;
            return Response::from_parts(parts, Body::from(bytes));
        }
        return Response::from_parts(parts, Body::empty());
    }
    response
}

/// Replay a micro-cached response like PHP's `MicroCaching`: only the JSON
/// body is reused; the freshly built response gets default headers (Symfony
/// then adds `no-cache, private`), so cache flags from the original handler
/// response are intentionally dropped.
fn rebuild(cached: CachedResponse) -> Response {
    let mut response = Response::new(Body::from(cached.body));
    *response.status_mut() = cached.status;
    if let Some(content_type) = cached.headers.get(header::CONTENT_TYPE) {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type.clone());
    }
    response
}

/// Token-bucket-ish rate limiter using fixed windows, used only when
/// `KUUKAN_RATE_LIMIT=true` (the public API enforces 3/s and 60/min).
#[derive(Clone)]
pub struct RateLimiter {
    pub enabled: bool,
    pub per_second: u64,
    pub per_minute: u64,
    second: Cache<String, Arc<AtomicU64>>,
    minute: Cache<String, Arc<AtomicU64>>,
}

impl RateLimiter {
    pub fn new(enabled: bool, per_second: u64, per_minute: u64) -> Self {
        RateLimiter {
            enabled,
            per_second,
            per_minute,
            second: Cache::builder()
                .time_to_live(Duration::from_secs(1))
                .max_capacity(100_000)
                .build(),
            minute: Cache::builder()
                .time_to_live(Duration::from_secs(60))
                .max_capacity(100_000)
                .build(),
        }
    }
}

fn client_key(req: &Request) -> String {
    if let Some(forwarded) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        return forwarded
            .split(',')
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string();
    }
    req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    req: Request,
    next: Next,
) -> Response {
    if !limiter.enabled {
        return next.run(req).await;
    }
    let key = client_key(&req);
    let (second_count, minute_count) = {
        let s = limiter
            .second
            .get_with(key.clone(), async { Arc::new(AtomicU64::new(0)) })
            .await;
        let m = limiter
            .minute
            .get_with(key.clone(), async { Arc::new(AtomicU64::new(0)) })
            .await;
        (
            s.fetch_add(1, Ordering::Relaxed) + 1,
            m.fetch_add(1, Ordering::Relaxed) + 1,
        )
    };

    if second_count > limiter.per_second || minute_count > limiter.per_minute {
        let now = chrono::Utc::now().timestamp() as u64;
        let reset = if second_count > limiter.per_second {
            now + 1
        } else {
            now + 60
        };
        return ApiErrorResponse(ApiError::RateLimited {
            retry_after_secs: reset.saturating_sub(now),
            limit: limiter.per_minute,
            remaining: 0,
            reset_epoch: reset,
        })
        .into_response();
    }
    next.run(req).await
}

/// CORS layer mirroring `config/cors.php`.
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};
    CorsLayer::new()
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any)
        .max_age(Duration::from_secs(86_400))
}

/// `SourceHeartbeatMonitor` middleware: every incoming request counts as a
/// healthy source sample, exactly like the PHP event dispatch. The write is
/// spawned so it never adds latency to the response path.
pub async fn source_health_middleware(
    State(state): State<crate::state::AppState>,
    req: Request,
    next: Next,
) -> Response {
    let store = state.store.clone();
    tokio::spawn(async move {
        let _ = store
            .record_source_result(true, kuukan_store::now_unix())
            .await;
    });
    next.run(req).await
}

/// Response middleware: Symfony/Laravel set `Cache-Control: no-cache, private`
/// on any response that did not configure caching. Endpoints that add Jikan
/// cache flags set their own header first, so this only touches plain
/// responses (errors, root metadata, repository-backed endpoints, micro-cache
/// replays).
pub async fn default_cache_control_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    if !response.headers().contains_key(header::CACHE_CONTROL) {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache, private"),
        );
    }
    response
}

/// Small helper used by handlers/tests to build a cached response.
pub fn json_bytes(value: &serde_json::Value) -> Bytes {
    Bytes::from(serde_json::to_vec(value).unwrap_or_default())
}

#[allow(dead_code)]
fn _assert_header_value() {
    let _: Option<HeaderValue> = None;
    let _ = header::CONTENT_TYPE;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_type_extracts_second_segment_under_v1() {
        assert_eq!(request_type("/v1/anime/1"), "anime");
        assert_eq!(request_type("/v1/users/neko"), "users");
        assert_eq!(request_type("/anime/1"), "anime");
        assert_eq!(request_type("/"), "");
    }

    #[test]
    fn no_caching_paths() {
        assert!(is_no_caching("/v1/random/anime"));
        assert!(is_no_caching("/v1/insights"));
        assert!(!is_no_caching("/v1/anime/1"));
    }
}
