//! Stored payload helpers.
//!
//! Jikan stores scraped results in per-endpoint documents shaped like the
//! parser models, e.g.:
//!
//! - search/list: `{"results": [...], "last_visible_page": 1, "has_next_page": false}`
//! - anime characters+staff: `{"characters": [...], "staff": [...]}`
//! - episodes: `{"episodes": [...], "last_visible_page": N, "has_next_page": bool}`
//!
//! Kuukan stores the same payloads (as JSON) so API rendering and cache
//! semantics match. These helpers build and read the common shapes.

use serde_json::Value;

pub fn get_results(payload: &Value) -> &[Value] {
    payload
        .get("results")
        .and_then(Value::as_array)
        .map(|v| v.as_slice())
        .unwrap_or(&[])
}

pub fn last_visible_page(payload: &Value) -> u64 {
    payload
        .get("last_visible_page")
        .and_then(Value::as_u64)
        .unwrap_or(1)
}

pub fn has_next_page(payload: &Value) -> bool {
    payload
        .get("has_next_page")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}
