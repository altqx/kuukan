//! Response rendering helpers: cache headers and JSON envelopes.
//!
//! Mirrors `App\Macros\ResponseJikanCacheFlags` / `App\Support\CachedData`:
//! - `X-Request-Fingerprint: request:<type>:<sha1(uri)>`
//! - `Cache-Control: public, s-maxage=<ttl>` (Symfony `setTtl`)
//! - `Expires: <last_modified + ttl>` in IMF-fixdate (Symfony `setExpires`)
//! - `Last-Modified: <last_modified>` in IMF-fixdate

use axum::body::Body;
use axum::http::{HeaderValue, Response};
use axum::response::IntoResponse;
use chrono::{DateTime, TimeZone, Utc};

/// IMF-fixdate, e.g. `Sun, 06 Nov 1994 08:49:37 GMT`.
pub fn httpdate(timestamp: i64) -> String {
    let dt: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(Utc::now);
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Attach the Jikan cache flags to a response.
///
/// `last_modified` is the cached document's `modifiedAt` (unix seconds); when
/// there is no cached document PHP passes 0 so `Last-Modified` is the epoch.
pub fn attach_cache_flags(
    response: &mut Response<Body>,
    fingerprint: &str,
    last_modified: i64,
    ttl: u64,
) {
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(fingerprint) {
        headers.insert("X-Request-Fingerprint", value);
    }
    if let Ok(value) = HeaderValue::from_str(&format!("public, s-maxage={ttl}")) {
        headers.insert("Cache-Control", value);
    }
    let expiry = last_modified.saturating_add(ttl as i64);
    if let Ok(value) = HeaderValue::from_str(&httpdate(expiry)) {
        headers.insert("Expires", value);
    }
    if let Ok(value) = HeaderValue::from_str(&httpdate(last_modified)) {
        headers.insert("Last-Modified", value);
    }
}

/// Build a JSON response with cache flags in one call.
pub fn json_with_cache_flags(
    value: serde_json::Value,
    fingerprint: &str,
    last_modified: i64,
    ttl: u64,
) -> Response<Body> {
    let mut response = axum::Json(value).into_response();
    attach_cache_flags(&mut response, fingerprint, last_modified, ttl);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn httpdate_matches_imf_fixdate() {
        // 784111777 = Sun, 06 Nov 1994 08:49:37 GMT
        assert_eq!(httpdate(784111777), "Sun, 06 Nov 1994 08:49:37 GMT");
    }

    #[tokio::test]
    async fn cache_flags_match_php_semantics() {
        let response = json_with_cache_flags(
            serde_json::json!({"data": {}}),
            "request:anime:abc",
            1000,
            3600,
        );
        assert_eq!(
            response.headers().get("Cache-Control").unwrap(),
            "public, s-maxage=3600"
        );
        assert_eq!(response.headers().get("X-Request-Fingerprint").unwrap(), "request:anime:abc");
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["data"], serde_json::json!({}));
    }
}
