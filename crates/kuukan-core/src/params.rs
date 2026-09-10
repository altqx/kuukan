//! Request query parameter handling.
//!
//! Jikan parses query strings with PHP `parse_str` semantics (later values win)
//! and validates them through spatie/laravel-data DTOs. `Query` captures the
//! raw string map plus typed accessors with PHP-compatible coercion rules so
//! every endpoint parses parameters the same way.

use std::collections::BTreeMap;

/// Raw query map. Built once per request by the HTTP layer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    values: BTreeMap<String, Vec<String>>,
}

/// Error returned by typed accessors, rendered as a ValidationException later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamError {
    pub field: String,
    pub message: String,
}

impl ParamError {
    pub fn invalid(field: &str) -> Self {
        ParamError {
            field: field.to_string(),
            message: format!("The {field} field is invalid."),
        }
    }
}

impl Query {
    pub fn new() -> Self {
        Query::default()
    }

    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (k, v) in pairs {
            values.entry(k.into()).or_default().push(v.into());
        }
        Query { values }
    }

    /// PHP `parse_str`: the last occurrence wins.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .and_then(|v| v.last())
            .map(|s| s.as_str())
    }

    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(|s| s.as_str())
    }

    /// Laravel boolean rule: accepts true/false, 1/0, "1"/"0", on/off, yes/no.
    pub fn get_bool(&self, key: &str) -> Result<Option<bool>, ParamError> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => match v.to_ascii_lowercase().as_str() {
                "1" | "true" | "on" | "yes" => Ok(Some(true)),
                "0" | "false" | "off" | "no" => Ok(Some(false)),
                _ => Err(ParamError::invalid(key)),
            },
        }
    }

    pub fn get_i64(&self, key: &str) -> Result<Option<i64>, ParamError> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => v
                .trim()
                .parse::<i64>()
                .map(Some)
                .map_err(|_| ParamError::invalid(key)),
        }
    }

    pub fn get_f64(&self, key: &str) -> Result<Option<f64>, ParamError> {
        match self.get(key) {
            None => Ok(None),
            Some(v) => v
                .trim()
                .parse::<f64>()
                .map(Some)
                .map_err(|_| ParamError::invalid(key)),
        }
    }

    /// `page` parameter: default 1, values < 1 are clamped to 1 by the paginator.
    pub fn page(&self) -> Result<u64, ParamError> {
        Ok(self.get_i64("page")?.unwrap_or(1).max(1) as u64)
    }

    /// `limit` parameter, capped at `max` (and never below 1).
    pub fn limit(&self, max: u64) -> Result<Option<u64>, ParamError> {
        match self.get_i64("limit")? {
            None => Ok(None),
            Some(v) => Ok(Some(v.clamp(1, max as i64) as u64)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_value_wins_like_parse_str() {
        let q = Query::from_pairs([("q", "first"), ("q", "second")]);
        assert_eq!(q.get("q"), Some("second"));
    }

    #[test]
    fn booleans_follow_laravel() {
        assert_eq!(
            Query::from_pairs([("sfw", "true")]).get_bool("sfw"),
            Ok(Some(true))
        );
        assert_eq!(
            Query::from_pairs([("sfw", "0")]).get_bool("sfw"),
            Ok(Some(false))
        );
        assert!(Query::from_pairs([("sfw", "maybe")])
            .get_bool("sfw")
            .is_err());
    }

    #[test]
    fn pagination_defaults() {
        let q = Query::new();
        assert_eq!(q.page().unwrap(), 1);
        assert_eq!(q.limit(25).unwrap(), None);
        let q = Query::from_pairs([("limit", "999")]);
        assert_eq!(q.limit(25).unwrap(), Some(25));
    }
}
