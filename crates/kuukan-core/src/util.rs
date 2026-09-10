//! Shared utilities: request fingerprints, PHP-style date formatting and
//! compatibility helpers used across crates.

use chrono::{DateTime, FixedOffset};
use sha1::{Digest, Sha1};

/// PHP `sha1($value)` as lowercase hex.
pub fn sha1_hex(value: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `HttpHelper::resolveRequestFingerprint()`:
/// `request:<type>:<sha1(request_uri)>`.
pub fn request_fingerprint(request_type: &str, uri: &str) -> String {
    format!("request:{}:{}", request_type, sha1_hex(uri))
}

/// Map a kuukan `/v1` URI to the upstream `/v4` form used for cache keys.
///
/// Kuukan serves only `/v1`, but fingerprints (cache documents and the
/// `X-Request-Fingerprint` header) hash the equivalent Jikan URI so an
/// existing Jikan cache can be imported untouched.
pub fn upstream_uri(uri: &str) -> String {
    match uri.strip_prefix("/v1") {
        Some(rest) => format!("/v4{rest}"),
        None => uri.to_string(),
    }
}

/// Jikan-compatible request fingerprint: hashes the upstream `/v4` URI.
pub fn jikan_request_fingerprint(request_type: &str, uri: &str) -> String {
    request_fingerprint(request_type, &upstream_uri(uri))
}

/// PHP `DATE_ATOM` (`Y-m-d\TH:i:sP`), e.g. `2024-01-06T00:00:00+00:00`.
pub fn format_atom(dt: &DateTime<FixedOffset>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// `DATE_ATOM` for UTC values, preserving the `+00:00` suffix PHP prints.
pub fn utc_atom(dt: &DateTime<chrono::Utc>) -> String {
    format_atom(&dt.with_timezone(&FixedOffset::east_opt(0).unwrap()))
}

/// Jikan renders an empty `related` map as `{}` and a non-empty one as
/// `[{relation, entry}]` for anime/manga. This mirrors
/// `HttpHelper::serializeEmptyObjectsControllerLevel`.
pub fn normalize_related(related: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};
    match related {
        Value::Object(map) if map.is_empty() => Value::Object(Map::new()),
        Value::Object(map) => {
            let mut out = Vec::with_capacity(map.len());
            for (relation, entries) in map {
                out.push(serde_json::json!({
                    "relation": relation,
                    "entry": entries,
                }));
            }
            Value::Array(out)
        }
        Value::Array(_) => related.clone(),
        Value::Null => Value::Object(Map::new()),
        other => other.clone(),
    }
}

/// `MAX_RESULTS_PER_PAGE` from the environment (default 25).
pub fn max_results_per_page() -> u64 {
    std::env::var("MAX_RESULTS_PER_PAGE")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| *v > 0)
        .unwrap_or(25)
}

/// `SOURCE_TIMEOUT` in seconds (jikan's `.env.dist` default is 10).
pub fn source_timeout_secs() -> u64 {
    std::env::var("SOURCE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn fingerprint_matches_php_format() {
        // sha1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
        assert_eq!(
            request_fingerprint("anime", ""),
            "request:anime:da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        // sha1("/v1/anime/1")
        assert_eq!(
            request_fingerprint("anime", "/v1/anime/1"),
            "request:anime:5dd35ef96cd8f51cbfb167f8a1ab3d2a4ba46839"
        );
    }

    #[test]
    fn atom_matches_php_date_atom() {
        let dt = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 6, 0, 0, 0)
            .unwrap();
        assert_eq!(format_atom(&dt), "2024-01-06T00:00:00+00:00");
        let jst = FixedOffset::east_opt(9 * 3600)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 6, 0, 0, 0)
            .unwrap();
        assert_eq!(format_atom(&jst), "2024-01-06T00:00:00+09:00");
    }
}
