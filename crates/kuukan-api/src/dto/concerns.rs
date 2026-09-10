//! Shared DTO parsing helpers.
//!
//! Port of `app/Dto/Concerns/*` combined with spatie/laravel-data's
//! `PreparesData::prepareForPipeline` semantics and the pipeline order observed
//! in laravel-data 3.11:
//!
//! 1. `MapPropertiesDataPipe` (input name mapping, e.g. `order_by`)
//! 2. `ValidatePropertiesDataPipe` (validation)
//! 3. `DefaultValuesDataPipe` (property defaults)
//! 4. `CastPropertiesDataPipe` (enum/date casts)
//!
//! Because validation runs *before* casting, invalid enum/date/bool values are
//! 400 ValidationExceptions, and the `ContextualBooleanCast` (which would turn
//! an empty string into `true`) is only reached when validation passed — in
//! practice `?sfw` with no value fails the Laravel `boolean` rule. Kuukan
//! reproduces that behavior; the differential harness tracks it.

use kuukan_core::error::ApiError;
use kuukan_core::params::Query;
use kuukan_core::util::max_results_per_page;

use crate::dto::validation::Validator;

/// Parse a boolean flag (`sfw`, `unapproved`, `kids`, `spoilers`, ...).
///
/// `PreparesData` converts the strings `"true"`/`"false"` before validation;
/// afterwards only Laravel's `boolean` rule values pass (`true`, `false`, `1`,
/// `0`, `"1"`, `"0"`).
pub fn bool_flag(query: &Query, field: &str, default: bool) -> Result<bool, ApiError> {
    Ok(optional_bool_flag(query, field)?.unwrap_or(default))
}

/// Nullable boolean flag (e.g. `unapproved`, `kids`).
pub fn optional_bool_flag(query: &Query, field: &str) -> Result<Option<bool>, ApiError> {
    let Some(raw) = query.get(field) else {
        return Ok(None);
    };
    // `ContextualBooleanCast`: an empty value means "true" for flags like
    // `?sfw` / `?unapproved` / `?kids`.
    if raw.is_empty() {
        return Ok(Some(true));
    }
    let prepared = match raw {
        "true" => "1",
        "false" => "0",
        other => other,
    };
    match prepared {
        "1" => Ok(Some(true)),
        "0" => Ok(Some(false)),
        _ => {
            let mut v = Validator::new();
            v.add(field, format!("The {} field must be true or false.", field));
            Err(v.finish().unwrap_err())
        }
    }
}

/// `page` parameter: `#[Numeric, Min(1)] public int|Optional $page = 1`.
pub fn page(query: &Query) -> Result<u64, ApiError> {
    let Some(raw) = query.get("page") else {
        return Ok(1);
    };
    let mut v = Validator::new();
    let numeric = v.numeric("page", raw);
    if numeric {
        let value: f64 = raw.trim().parse().unwrap_or(0.0);
        v.min_numeric("page", value, 1.0);
    }
    v.finish()?;
    Ok(raw.trim().parse::<f64>().unwrap_or(1.0).max(1.0) as u64)
}

/// `limit` parameter:
/// `#[IntegerType, Min(1), MaxLimitWithFallback]`, defaulted to
/// `max_results_per_page()` when absent.
pub fn limit(query: &Query, default_limit: Option<u64>) -> Result<u64, ApiError> {
    let Some(raw) = query.get("limit") else {
        return Ok(default_limit.unwrap_or_else(max_results_per_page));
    };
    let mut v = Validator::new();
    let integer = v.integer("limit", raw);
    if integer {
        let value: f64 = raw.trim().parse().unwrap_or(0.0);
        v.min_numeric("limit", value, 1.0);
    } else if raw.is_empty() {
        // Laravel's `min` rule on a non-numeric string validates length.
        v.add("limit", "The limit field must be at least 1.".to_string());
    }
    v.max_results_per_page("limit", raw, max_results_per_page());
    v.finish()?;
    Ok(raw.trim().parse::<i64>().unwrap_or(1).max(1) as u64)
}

/// `DateFormat("Y-m-d")` + `Sometimes|Required`, returned as unix seconds.
pub fn date_param(query: &Query, field: &str) -> Result<Option<i64>, ApiError> {
    let Some(raw) = query.get(field) else {
        return Ok(None);
    };
    if raw.is_empty() {
        // PreparesData: empty string for optional non-bool becomes missing.
        return Ok(None);
    }
    let mut v = Validator::new();
    if !v.date_format(field, raw, "Y-m-d") {
        return Err(v.finish().unwrap_err());
    }
    let ts = crate::dto::validation::parse_date(raw, "Y-m-d");
    Ok(ts)
}

/// `max_score`/`min_score`/`score` numeric values parsed without range checks
/// (DTOs apply `between` themselves; the ranges differ per endpoint).
pub fn numeric_param(query: &Query, field: &str) -> Result<Option<f64>, ApiError> {
    let Some(raw) = query.get(field) else {
        return Ok(None);
    };
    let mut v = Validator::new();
    if !v.numeric(field, raw) {
        return Err(v.finish().unwrap_err());
    }
    Ok(raw.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_defaults_and_validates() {
        assert_eq!(page(&Query::new()).unwrap(), 1);
        let q = Query::from_pairs([("page", "3")]);
        assert_eq!(page(&q).unwrap(), 3);
        let q = Query::from_pairs([("page", "abc")]);
        assert!(page(&q).is_err());
        let q = Query::from_pairs([("page", "0")]);
        let err = page(&q).unwrap_err();
        assert_eq!(
            err.body(false)["messages"]["page"][0],
            "The page must be at least 1."
        );
    }

    #[test]
    fn limit_follows_max_results_rule() {
        assert_eq!(limit(&Query::new(), None).unwrap(), 25);
        let q = Query::from_pairs([("limit", "10")]);
        assert_eq!(limit(&q, None).unwrap(), 10);
        let q = Query::from_pairs([("limit", "999")]);
        let err = limit(&q, None).unwrap_err();
        assert_eq!(
            err.body(false)["messages"]["limit"][0],
            "Value 999 is higher than the configured '25' max value."
        );
    }

    #[test]
    fn bool_flags_match_php() {
        assert!(bool_flag(&Query::from_pairs([("sfw", "true")]), "sfw", false).unwrap());
        assert!(!bool_flag(&Query::new(), "sfw", false).unwrap());
        assert_eq!(
            optional_bool_flag(&Query::from_pairs([("kids", "1")]), "kids").unwrap(),
            Some(true)
        );
        // `?sfw=` (empty value): `ContextualBooleanCast` turns it into true.
        assert_eq!(
            optional_bool_flag(&Query::from_pairs([("sfw", "")]), "sfw").unwrap(),
            Some(true)
        );
        assert!(optional_bool_flag(&Query::from_pairs([("sfw", "maybe")]), "sfw").is_err());
    }

    #[test]
    fn dates_parse_y_m_d() {
        let q = Query::from_pairs([("start_date", "2020-01-02")]);
        assert!(date_param(&q, "start_date").unwrap().is_some());
        let q = Query::from_pairs([("start_date", "2020-13-01")]);
        assert!(date_param(&q, "start_date").is_err());
    }
}
