//! HTTP error rendering: turns [`kuukan_core::error::ApiError`] into the exact
//! Jikan error response (body + status + rate limit headers).

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use kuukan_core::error::ApiError;

/// Wrapper allowing `?` to convert [`ApiError`] into an axum response in
/// handlers (`Result<Json<Value>, ApiErrorResponse>`).
#[derive(Debug)]
pub struct ApiErrorResponse(pub ApiError);

impl From<ApiError> for ApiErrorResponse {
    fn from(err: ApiError) -> Self {
        ApiErrorResponse(err)
    }
}

impl From<kuukan_mal::error::MalError> for ApiErrorResponse {
    fn from(err: kuukan_mal::error::MalError) -> Self {
        ApiErrorResponse(mal_error_to_api(err))
    }
}

/// Map a MAL client error to the exact Jikan error, shared by handlers and the
/// cached scrape service.
pub fn mal_error_to_api(err: kuukan_mal::error::MalError) -> ApiError {
    match err {
        kuukan_mal::error::MalError::BadResponse { status, url } => {
            ApiError::from_upstream_status(status, format!("{status} on {url}"))
        }
        kuukan_mal::error::MalError::Transport(_) => ApiError::UpstreamStatus {
            status: 500,
            kind: kuukan_core::error::UpstreamKind::Upstream,
            error: Some(err.to_string()),
        },
        kuukan_mal::error::MalError::Timeout { seconds, message } => ApiError::UpstreamTimeout {
            timeout_secs: seconds,
            error: Some(message),
        },
        kuukan_mal::error::MalError::Parse(message) => ApiError::parser(message),
        other => ApiError::internal(other.to_string()),
    }
}

/// Whether `APP_DEBUG` is enabled (controls cached-error details).
pub fn app_debug() -> bool {
    matches!(
        std::env::var("APP_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = self.0.body(app_debug());
        let mut response = axum::Json(body).into_response();
        *response.status_mut() = status;
        for (name, value) in self.0.headers() {
            if let (Ok(name), Ok(value)) = (
                header::HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(&value),
            ) {
                response.headers_mut().insert(name, value);
            }
        }
        response
    }
}

/// Shared response helper for handlers that already produced a JSON value.
pub fn json_ok(value: serde_json::Value) -> Response {
    axum::Json(value).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn not_found_renders_php_body() {
        let response = ApiErrorResponse(ApiError::not_found()).into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["type"], "HttpException");
        assert_eq!(body["message"], "Not Found");
    }
}
