//! Request extractors.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use kuukan_core::params::Query;

use crate::error::ApiErrorResponse;

/// Raw query-string extractor preserving repeated keys with PHP `parse_str`
/// semantics (last value wins) via [`Query`].
///
/// `#[derive(Deserialize)]` extractors cannot express Jikan's rules (enums with
/// aliases, PHP coercion), so handlers take `RawQuery` and run a DTO
/// `parse`/validation function.
#[derive(Debug, Clone, Default)]
pub struct RawQuery(pub Query);

impl<S> FromRequestParts<S> for RawQuery
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or("");
        let pairs = form_urlencoded::parse(query.as_bytes())
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect::<Vec<_>>();
        Ok(RawQuery(Query::from_pairs(pairs)))
    }
}

/// Laravel route constraints are `[0-9]+`: a non-numeric id never matches the
/// route and renders as `HttpException` 404, not axum's `Path<i64>` 400. Digit
/// strings that overflow `i64` saturate like PHP's numeric string cast.
pub fn route_id(value: &str) -> Result<i64, ApiErrorResponse> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiErrorResponse(kuukan_core::error::ApiError::not_found()));
    }
    Ok(value.parse::<i64>().unwrap_or(i64::MAX))
}
