//! API error model.
//!
//! Ported from `app/Exceptions/Handler.php` in jikan-rest. Every variant
//! renders the exact same JSON body and status code as the PHP exception
//! handler, so API clients cannot tell the implementations apart.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// The upstream parser/API base for GitHub report URLs.
const JIKAN_PARSER_REPO: &str = "jikan-me/jikan";
const JIKAN_REST_REPO: &str = "jikan-me/jikan-rest";

/// HTTP status text, mirroring `Symfony\Component\HttpFoundation\Response::$statusTexts`.
pub fn status_text(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        306 => "Switch Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "Unknown Status",
    }
}

/// A GitHub issue report URL, mirroring `App\Exceptions\GithubReport`.
pub fn github_report_url(repo: &str, title: &str, body: &str, request: Option<&str>) -> String {
    let mut url = format!(
        "https://github.com/{repo}/issues/new?title={}&body={}",
        urlencoding(title),
        urlencoding(body)
    );
    if let Some(request) = request {
        url.push_str(&format!("&request={}", urlencoding(request)));
    }
    url
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Error kinds rendered by the HTTP layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// 400 `ValidationException` from DTO validation rules.
    Validation {
        messages: BTreeMap<String, Vec<String>>,
    },
    /// 400 `BadRequestException` thrown by controllers.
    BadRequest { message: String },
    /// `HttpException`, rendered with the status text as message (e.g. 404 Not Found).
    Http { status: u16 },
    /// MAL returned an error status: `BadResponseException` / `RateLimitException` /
    /// `UpstreamException`, mirroring the PHP switch in Handler::render.
    UpstreamStatus {
        status: u16,
        kind: UpstreamKind,
        error: Option<String>,
    },
    /// Request to MAL timed out.
    UpstreamTimeout {
        timeout_secs: u64,
        error: Option<String>,
    },
    /// Parser failure (`ParserException`).
    Parser {
        error: Option<String>,
        report_url: Option<String>,
    },
    /// Storage/cache connection failure (PHP: Redis ConnectionException).
    Storage {
        error: Option<String>,
        report_url: Option<String>,
    },
    /// Unhandled exception.
    Internal {
        message: String,
        trace: Option<String>,
        error: Option<String>,
        report_url: Option<String>,
    },
    /// Optional public-API style rate limiting.
    RateLimited {
        retry_after_secs: u64,
        limit: u64,
        remaining: u64,
        reset_epoch: u64,
    },
}

/// Classified MAL upstream failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamKind {
    NotFound,
    RateLimited,
    Upstream,
    Other,
    Timeout,
}

impl ApiError {
    pub fn not_found() -> Self {
        ApiError::Http { status: 404 }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        ApiError::BadRequest {
            message: message.into(),
        }
    }

    /// Build from a MAL response status code, exactly like Handler::render.
    pub fn from_upstream_status(status: u16, error: impl Into<String>) -> Self {
        let error = error.into();
        let (status, kind) = match status {
            404 => (404, UpstreamKind::NotFound),
            429 => (429, UpstreamKind::RateLimited),
            403 | 500 | 501 | 502 | 503 | 504 => (500, UpstreamKind::Upstream),
            other => (other, UpstreamKind::Other),
        };
        ApiError::UpstreamStatus {
            status,
            kind,
            error: Some(error),
        }
    }

    pub fn parser(error: impl Into<String>) -> Self {
        let error = error.into();
        let report_url = github_report_url(JIKAN_PARSER_REPO, "ParserException", &error, None);
        ApiError::Parser {
            error: Some(error),
            report_url: Some(report_url),
        }
    }

    pub fn internal(error: impl Into<String>) -> Self {
        let error = error.into();
        let report_url = github_report_url(JIKAN_REST_REPO, "Exception", &error, None);
        ApiError::Internal {
            message: "Unhandled Exception. Please follow report_url to generate an issue on GitHub"
                .to_string(),
            trace: None,
            error: Some(error),
            report_url: Some(report_url),
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            ApiError::Validation { .. } => 400,
            ApiError::BadRequest { .. } => 400,
            ApiError::Http { status } => *status,
            ApiError::UpstreamStatus { status, .. } => *status,
            ApiError::UpstreamTimeout { .. } => 500,
            ApiError::Parser { .. } => 500,
            ApiError::Storage { .. } => 500,
            ApiError::Internal { .. } => 500,
            ApiError::RateLimited { .. } => 429,
        }
    }

    /// Whether the HTTP layer should print the internal error string.
    /// Mirrors `env('APP_DEBUG')` gating in the PHP handler.
    pub fn body(&self, debug: bool) -> Value {
        match self {
            ApiError::Validation { messages } => json!({
                "status": 400,
                "type": "ValidationException",
                "messages": messages,
                "error": "Invalid or incomplete request. Make sure your request is correct. https://docs.api.jikan.moe/",
            }),
            ApiError::BadRequest { message } => json!({
                "status": 400,
                "type": "BadRequestException",
                "message": message,
                "error": null,
            }),
            ApiError::Http { status } => json!({
                "status": status,
                "type": "HttpException",
                "message": status_text(*status),
                "error": null,
            }),
            ApiError::UpstreamStatus {
                status,
                kind,
                error,
            } => match kind {
                UpstreamKind::NotFound => json!({
                    "status": status,
                    "type": "BadResponseException",
                    "message": "Resource does not exist",
                    "error": error,
                }),
                UpstreamKind::RateLimited => json!({
                    "status": status,
                    "type": "RateLimitException",
                    "message": "Jikan is being rate limited by MyAnimeList.",
                    "error": error,
                }),
                UpstreamKind::Upstream | UpstreamKind::Timeout => json!({
                    "status": 500,
                    "type": "UpstreamException",
                    "message": "Request to MyAnimeList.net failed. MyAnimeList.net may be down/unavailable, refuses to connect or took too long to respond. Please try again later.",
                    "error": error,
                }),
                UpstreamKind::Other => json!({
                    "status": status,
                    "type": "BadResponseException",
                    "message": "Something went wrong, please try again later.",
                    "error": error,
                }),
            },
            ApiError::UpstreamTimeout {
                timeout_secs,
                error,
            } => json!({
                "status": 500,
                "type": "UpstreamException",
                "message": format!("Request to MyAnimeList.net timed out ({timeout_secs} seconds). Please try again later."),
                "error": error,
            }),
            ApiError::Parser { error, report_url } => json!({
                "status": 500,
                "type": "ParserException",
                "message": "Unable to parse this request. Please follow report_url to generate an issue on GitHub",
                "error": error,
                "report_url": report_url,
            }),
            ApiError::Storage { error, report_url } => json!({
                "status": 500,
                "type": "ConnectionException",
                "message": "Failed to communicate with the Redis server",
                "error": if debug { error.clone() } else { None },
                "report_url": report_url,
            }),
            ApiError::Internal {
                message,
                trace,
                error,
                report_url,
            } => json!({
                "status": 500,
                "type": "Exception",
                "message": message,
                "trace": trace,
                "error": error,
                "report_url": report_url,
            }),
            ApiError::RateLimited {
                retry_after_secs,
                limit,
                remaining,
                reset_epoch,
            } => {
                let mut map = Map::new();
                map.insert("status".into(), json!(429));
                map.insert("type".into(), json!("RateLimitException"));
                map.insert(
                    "message".into(),
                    json!("You are being rate limited. Please try again later."),
                );
                map.insert("error".into(), Value::Null);
                map.insert("retry_after".into(), json!(retry_after_secs));
                map.insert("limit".into(), json!(limit));
                map.insert("remaining".into(), json!(remaining));
                map.insert("reset".into(), json!(reset_epoch));
                Value::Object(map)
            }
        }
    }

    /// Response headers that accompany this error.
    pub fn headers(&self) -> Vec<(String, String)> {
        match self {
            ApiError::RateLimited {
                retry_after_secs,
                limit,
                remaining,
                reset_epoch,
            } => vec![
                ("Retry-After".to_string(), retry_after_secs.to_string()),
                ("X-RateLimit-Limit".to_string(), limit.to_string()),
                ("X-RateLimit-Remaining".to_string(), remaining.to_string()),
                ("X-RateLimit-Reset".to_string(), reset_epoch.to_string()),
            ],
            _ => Vec::new(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_body_matches_php() {
        let body = ApiError::not_found().body(false);
        assert_eq!(
            body,
            json!({"status":404,"type":"HttpException","message":"Not Found","error":null})
        );
    }

    #[test]
    fn mal_404_body_matches_php() {
        let body = ApiError::from_upstream_status(404, "404 on https://myanimelist.net/anime/1")
            .body(false);
        assert_eq!(
            body,
            json!({
                "status":404,
                "type":"BadResponseException",
                "message":"Resource does not exist",
                "error":"404 on https://myanimelist.net/anime/1"
            })
        );
    }

    #[test]
    fn mal_503_becomes_upstream_500() {
        let err = ApiError::from_upstream_status(503, "boom");
        assert_eq!(err.status(), 500);
        let body = err.body(false);
        assert_eq!(body["type"], "UpstreamException");
        assert_eq!(body["status"], 500);
    }

    #[test]
    fn validation_envelope() {
        let mut messages = BTreeMap::new();
        messages.insert(
            "q".to_string(),
            vec!["The q field is required.".to_string()],
        );
        let body = ApiError::Validation { messages }.body(false);
        assert_eq!(body["type"], "ValidationException");
        assert_eq!(body["messages"]["q"][0], "The q field is required.");
    }

    #[test]
    fn storage_hides_error_unless_debug() {
        let err = ApiError::Storage {
            error: Some("secret".into()),
            report_url: None,
        };
        assert_eq!(err.body(false)["error"], Value::Null);
        assert_eq!(err.body(true)["error"], "secret");
    }
}
