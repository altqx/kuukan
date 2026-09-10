//! Response envelope helpers.
//!
//! Jikan responses are always either `{"data": ...}` or
//! `{"pagination": {...}, "data": ...}`. These helpers keep the key order and
//! shapes consistent across resources. (JSON object key order is not observable
//! to compliant clients, but keeping the same order helps byte-level diffing.)

use crate::pagination::Pagination;
use serde_json::{json, Value};

/// `{"data": ...}` envelope.
pub fn data(data: impl Into<Value>) -> Value {
    json!({ "data": data.into() })
}

/// `{"pagination": ..., "data": ...}` envelope.
pub fn paged(pagination: &Pagination, data: impl Into<Value>) -> Value {
    json!({
        "pagination": serde_json::to_value(pagination).unwrap_or(Value::Null),
        "data": data.into(),
    })
}

/// Map a list of items, keeping the envelope.
pub fn paged_items<T: serde::Serialize>(pagination: &Pagination, items: &[T]) -> Value {
    let items: Vec<Value> = items
        .iter()
        .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
        .collect();
    paged(pagination, items)
}
