//! Validation helpers mirroring Laravel's validator and spatie/laravel-data
//! rules used by Jikan DTOs.
//!
//! The resulting message bag is rendered verbatim in the API's
//! `ValidationException` body, so message templates must match Laravel's
//! `en/validation.php` exactly.

use kuukan_core::error::ApiError;
use std::collections::BTreeMap;

/// Collects validation messages per field.
#[derive(Debug, Default)]
pub struct Validator {
    messages: BTreeMap<String, Vec<String>>,
}

/// Laravel renders attribute names by replacing `_` and `.` with spaces.
pub fn attribute(field: &str) -> String {
    field.replace(['_', '.'], " ")
}

impl Validator {
    pub fn new() -> Self {
        Validator::default()
    }

    pub fn add(&mut self, field: &str, message: String) {
        self.messages
            .entry(field.to_string())
            .or_default()
            .push(message);
    }

    fn required_message(&mut self, field: &str) {
        self.add(field, format!("The {} field is required.", attribute(field)));
    }

    fn type_message(&mut self, field: &str, kind: &str) {
        // Lumen's validation lang omits "field" for type rules.
        self.add(
            field,
            format!("The {} must be {kind}.", attribute(field)),
        );
    }

    /// `required`
    pub fn required(&mut self, field: &str, present: bool) {
        if !present {
            self.required_message(field);
        }
    }

    /// `required` when a value is `None`.
    pub fn required_value<T>(&mut self, field: &str, value: Option<&T>) -> bool {
        if value.is_none() {
            self.required_message(field);
            false
        } else {
            true
        }
    }

    /// `integer`
    pub fn integer(&mut self, field: &str, raw: &str) -> bool {
        if raw.trim().parse::<i64>().is_ok() {
            true
        } else {
            self.type_message(field, "an integer");
            false
        }
    }

    /// `numeric`
    pub fn numeric(&mut self, field: &str, raw: &str) -> bool {
        if raw.trim().parse::<f64>().is_ok() {
            true
        } else {
            self.type_message(field, "a number");
            false
        }
    }

    /// `string`
    pub fn string(&mut self, field: &str, _raw: &str) -> bool {
        // Query values are always strings; this rule can never fail here.
        let _ = field;
        true
    }

    /// `boolean` (Laravel accepts true/false/1/0/"1"/"0")
    pub fn boolean(&mut self, field: &str, raw: &str) -> bool {
        if matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "0" | "true" | "false"
        ) {
            true
        } else {
            self.add(
                field,
                format!("The {} field must be true or false.", attribute(field)),
            );
            false
        }
    }

    /// `min:N` for numeric values.
    pub fn min_numeric(&mut self, field: &str, value: f64, min: f64) {
        if value < min {
            self.add(
                field,
                format!(
                    "The {} must be at least {}.",
                    attribute(field),
                    trim_number(min)
                ),
            );
        }
    }

    /// `max:N` for numeric values.
    pub fn max_numeric(&mut self, field: &str, value: f64, max: f64) {
        if value > max {
            self.add(
                field,
                format!(
                    "The {} must not be greater than {}.",
                    attribute(field),
                    trim_number(max)
                ),
            );
        }
    }

    /// `between:a,b` for numeric values.
    pub fn between_numeric(&mut self, field: &str, value: f64, min: f64, max: f64) {
        if value < min || value > max {
            self.add(
                field,
                format!(
                    "The {} must be between {} and {}.",
                    attribute(field),
                    trim_number(min),
                    trim_number(max)
                ),
            );
        }
    }

    /// `size:N` for numeric values.
    pub fn size_numeric(&mut self, field: &str, value: f64, size: f64) {
        if (value - size).abs() > f64::EPSILON {
            self.add(
                field,
                format!(
                    "The {} must be {}.",
                    attribute(field),
                    trim_number(size)
                ),
            );
        }
    }

    /// `alpha`
    pub fn alpha(&mut self, field: &str, raw: &str) {
        if !raw.chars().all(|c| c.is_ascii_alphabetic()) {
            self.add(
                field,
                format!(
                    "The {} must only contain letters.",
                    attribute(field)
                ),
            );
        }
    }

    /// `date_format:...` — Jikan uses `Y-m-d` for start/end dates.
    pub fn date_format(&mut self, field: &str, raw: &str, format: &str) -> bool {
        if parse_date(raw, format).is_some() {
            true
        } else {
            self.add(
                field,
                format!(
                    "The {} does not match the format {}.",
                    attribute(field),
                    format
                ),
            );
            false
        }
    }

    /// `before_or_equal:field`
    pub fn before_or_equal(&mut self, field: &str, value: i64, other_field: &str, other: i64) {
        if value > other {
            self.add(
                field,
                format!(
                    "The {} must be a date before or equal to {}.",
                    attribute(field),
                    attribute(other_field)
                ),
            );
        }
    }

    /// `after_or_equal:field`
    pub fn after_or_equal(&mut self, field: &str, value: i64, other_field: &str, other: i64) {
        if value < other {
            self.add(
                field,
                format!(
                    "The {} must be a date after or equal to {}.",
                    attribute(field),
                    attribute(other_field)
                ),
            );
        }
    }

    /// `prohibits:other` — this field must not be present when another is.
    pub fn prohibits(&mut self, field: &str, other: &str, present: bool) {
        if present {
            self.add(
                field,
                format!(
                    "The {} field prohibits {} from being present.",
                    attribute(field),
                    attribute(other)
                ),
            );
        }
    }

    /// spatie/laravel-enum `EnumRule` message (lang line `enum::validation.enum`):
    /// `The {field} field is not a valid {php_class}.`
    pub fn enum_rule(
        &mut self,
        field: &str,
        raw: &str,
        accepted: &[&str],
        php_class: &str,
    ) -> bool {
        if accepted.iter().any(|v| *v == raw) {
            true
        } else {
            self.add(
                field,
                format!("The {} field is not a valid {php_class}.", attribute(field)),
            );
            false
        }
    }

    /// `MaxResultsPerPageRule` custom message, verbatim.
    pub fn max_results_per_page(&mut self, field: &str, raw: &str, max: u64) {
        let value = raw.trim();
        let is_numeric = value.parse::<f64>().is_ok();
        let over = value
            .parse::<i64>()
            .map(|v| v > max as i64)
            .unwrap_or(false);
        if !is_numeric || over {
            self.add(
                field,
                format!("Value {value} is higher than the configured '{max}' max value."),
            );
        }
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn messages(&self) -> &BTreeMap<String, Vec<String>> {
        &self.messages
    }

    /// Convert into the API error, if any messages were collected.
    pub fn finish(self) -> Result<(), ApiError> {
        if self.messages.is_empty() {
            Ok(())
        } else {
            Err(ApiError::Validation {
                messages: self.messages,
            })
        }
    }
}

fn trim_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Parse `Y-m-d` (and `Y-m-d H:i:s` used by a few DTOs) into unix days/seconds.
pub fn parse_date(raw: &str, format: &str) -> Option<i64> {
    use chrono::{NaiveDate, NaiveDateTime};
    match format {
        "Y-m-d" => NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp()),
        "Y-m-d H:i:s" => NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc().timestamp()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laravel_messages() {
        let mut v = Validator::new();
        v.required("q", false);
        v.integer("limit", "abc");
        v.min_numeric("limit", 0.0, 1.0);
        v.prohibits("score", "min_score", true);
        v.boolean("sfw", "maybe");
        v.enum_rule(
            "order_by",
            "bogus",
            &["mal_id", "title"],
            "App\\Enums\\AnimeOrderByEnum",
        );
        let messages = v.messages();
        assert_eq!(messages["q"][0], "The q field is required.");
        assert_eq!(messages["limit"][0], "The limit must be an integer.");
        assert_eq!(messages["limit"][1], "The limit must be at least 1.");
        assert_eq!(
            messages["score"][0],
            "The score field prohibits min score from being present."
        );
        assert_eq!(messages["sfw"][0], "The sfw field must be true or false.");
        assert_eq!(
            messages["order_by"][0],
            "The order by field is not a valid App\\Enums\\AnimeOrderByEnum."
        );
    }

    #[test]
    fn max_results_message_matches_php_rule() {
        let mut v = Validator::new();
        v.max_results_per_page("limit", "999", 25);
        assert_eq!(
            v.messages()["limit"][0],
            "Value 999 is higher than the configured '25' max value."
        );
    }

    #[test]
    fn attribute_names_use_spaces() {
        assert_eq!(attribute("min_score"), "min score");
        assert_eq!(attribute("start_date"), "start date");
    }
}
