//! URL builders for MyAnimeList requests.
//!
//! Each request type in the sibling modules implements [`MalRequest::path`],
//! mirroring `Jikan\Request\*\*::getPath()`. This module hosts the constants
//! and the `http_build_query()` equivalent shared by those builders.

pub mod anime;
pub mod character;
pub mod club;
pub mod forum;
pub mod genre;
pub mod magazine;
pub mod manga;
pub mod news;
pub mod person;
pub mod producer;
pub mod recommendations;
pub mod reviews;
pub mod schedule;
pub mod search;
pub mod season_list;
pub mod seasonal;
pub mod top;
pub mod user;
pub mod watch;

/// `Constants::BASE_URL`.
pub const BASE_URL: &str = "https://myanimelist.net";

/// `Constants::CDN_URL`.
pub const CDN_URL: &str = "https://cdn.myanimelist.net";

/// URL builder for a MyAnimeList request, mirroring Jikan\Request::getPath().
pub trait MalRequest {
    fn path(&self) -> String;
}

/// A value that can be passed to [`http_build_query`].
///
/// `None` is skipped entirely (PHP `http_build_query` drops `null` values),
/// booleans become `1`/`0`.
#[derive(Debug, Clone)]
pub enum QueryValue<'a> {
    None,
    Str(&'a str),
    Owned(String),
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
}

impl<'a> From<&'a str> for QueryValue<'a> {
    fn from(v: &'a str) -> Self {
        QueryValue::Str(v)
    }
}

impl From<String> for QueryValue<'_> {
    fn from(v: String) -> Self {
        QueryValue::Owned(v)
    }
}

impl<'a> From<Option<&'a str>> for QueryValue<'a> {
    fn from(v: Option<&'a str>) -> Self {
        match v {
            Some(v) => QueryValue::Str(v),
            None => QueryValue::None,
        }
    }
}

impl From<Option<String>> for QueryValue<'_> {
    fn from(v: Option<String>) -> Self {
        match v {
            Some(v) => QueryValue::Owned(v),
            None => QueryValue::None,
        }
    }
}

impl From<i64> for QueryValue<'_> {
    fn from(v: i64) -> Self {
        QueryValue::Int(v)
    }
}

impl From<i32> for QueryValue<'_> {
    fn from(v: i32) -> Self {
        QueryValue::Int(v as i64)
    }
}

impl From<u64> for QueryValue<'_> {
    fn from(v: u64) -> Self {
        QueryValue::UInt(v)
    }
}

impl From<u32> for QueryValue<'_> {
    fn from(v: u32) -> Self {
        QueryValue::UInt(v as u64)
    }
}

impl From<f64> for QueryValue<'_> {
    fn from(v: f64) -> Self {
        QueryValue::Float(v)
    }
}

impl From<bool> for QueryValue<'_> {
    fn from(v: bool) -> Self {
        QueryValue::Bool(v)
    }
}

/// Port of PHP `http_build_query($params)`: null values are skipped, values are
/// `urlencode()`d (space becomes `+`, `~` becomes `%7E`), the pairs keep their
/// insertion order.
pub fn http_build_query(params: &[(&str, QueryValue<'_>)]) -> String {
    let mut parts = Vec::with_capacity(params.len());
    for (key, value) in params {
        let value = match value {
            QueryValue::None => continue,
            QueryValue::Str(v) => (*v).to_string(),
            QueryValue::Owned(v) => v.clone(),
            QueryValue::Int(v) => v.to_string(),
            QueryValue::UInt(v) => v.to_string(),
            QueryValue::Float(v) => v.to_string(),
            QueryValue::Bool(v) => if *v { "1" } else { "0" }.to_string(),
        };
        parts.push(format!("{}={}", urlencode(key), urlencode(&value)));
    }
    parts.join("&")
}

/// PHP `urlencode()` (RFC 1738: space -> `+`, `~` percent-encoded, uppercase
/// hex).
pub fn urlencode(s: &str) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
    const URLENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_');
    utf8_percent_encode(s, URLENCODE_SET)
        .to_string()
        .replace("%20", "+")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected values captured from PHP 8.5 `http_build_query`/`urlencode`.
    #[test]
    fn http_build_query_matches_php() {
        let query = http_build_query(&[
            ("q", "a~b c&d".into()),
            ("type", 0.into()),
            ("show", QueryValue::None),
            ("score", 8.5.into()),
            ("flag", true.into()),
        ]);
        assert_eq!(query, "q=a%7Eb+c%26d&type=0&score=8.5&flag=1");
    }

    #[test]
    fn urlencode_matches_php() {
        assert_eq!(urlencode("~"), "%7E");
        assert_eq!(urlencode("-_. "), "-_.+");
        assert_eq!(urlencode("a b"), "a+b");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencode("ü"), "%C3%BC");
        assert_eq!(urlencode("日本"), "%E6%97%A5%E6%9C%AC");
        assert_eq!(urlencode("a+b"), "a%2Bb");
        assert_eq!(urlencode("%"), "%25");
    }

    #[test]
    fn optional_and_numeric_values() {
        assert_eq!(
            http_build_query(&[
                ("q", Some("x").into()),
                ("letter", None::<String>.into()),
                ("p", 12i64.into()),
                ("n", 7u32.into()),
                ("b", false.into()),
            ]),
            "q=x&p=12&n=7&b=0"
        );
    }
}
