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

use serde_json::{json, Value};

/// `{"results": [...], "last_visible_page": n, "has_next_page": bool}`
pub fn results_payload(results: Vec<Value>, last_visible_page: u64, has_next_page: bool) -> Value {
    json!({
        "results": results,
        "last_visible_page": last_visible_page,
        "has_next_page": has_next_page,
    })
}

/// Same as [`results_payload`] but for a named collection key
/// (e.g. `"characters"`, `"pictures"`, `"episodes"`).
pub fn keyed_payload(
    key: &str,
    items: Vec<Value>,
    last_visible_page: u64,
    has_next_page: bool,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(key.to_string(), Value::Array(items));
    map.insert("last_visible_page".into(), json!(last_visible_page));
    map.insert("has_next_page".into(), json!(has_next_page));
    Value::Object(map)
}

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
