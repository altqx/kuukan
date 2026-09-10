//! Request extractors.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use kuukan_core::params::Query;

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
