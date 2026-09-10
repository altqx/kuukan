//! HTTP client for MyAnimeList.
//!
//! Mirrors `Jikan\Goutte\GoutteWrapper`: any status `>= 400` becomes a
//! [`MalError::BadResponse`] carrying the status code and URL
//! (`"<status> on <url>"`). On top of that Kuukan retries transient failures
//! (429, 5xx, transport) with a small exponential backoff; 404 is never
//! retried.

use std::time::Duration;

use bytes::Bytes;

use crate::error::MalError;
use crate::parser::helper::HtmlDoc;

/// MAL client configuration, built from the jikan environment variables.
#[derive(Debug, Clone)]
pub struct MalConfig {
    /// `SOURCE_TIMEOUT` in seconds (default 10).
    pub timeout: Duration,
    /// `KUUKAN_MAL_PROXY` or `HTTPS_PROXY`.
    pub proxy: Option<String>,
    /// `KUUKAN_MAL_USER_AGENT` (default `kuukan/<crate version>`).
    pub user_agent: String,
    /// `KUUKAN_MAL_MAX_RETRIES` (default 0: same as jikan, which does not
    /// retry). Number of *retries*, not attempts.
    pub max_retries: u32,
}

impl Default for MalConfig {
    fn default() -> Self {
        MalConfig {
            timeout: Duration::from_secs(10),
            proxy: None,
            user_agent: default_user_agent(),
            max_retries: 0,
        }
    }
}

impl MalConfig {
    /// Read the same environment variables as jikan-rest's bootstrap + the
    /// `KUUKAN_*` overrides.
    pub fn from_env() -> Self {
        let timeout_secs = std::env::var("SOURCE_TIMEOUT")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(10);
        let proxy = std::env::var("KUUKAN_MAL_PROXY")
            .or_else(|_| std::env::var("HTTPS_PROXY"))
            .or_else(|_| std::env::var("https_proxy"))
            .ok()
            .filter(|v| !v.trim().is_empty());
        let user_agent = std::env::var("KUUKAN_MAL_USER_AGENT")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(default_user_agent);
        let max_retries = std::env::var("KUUKAN_MAL_MAX_RETRIES")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .unwrap_or(0);
        MalConfig {
            timeout: Duration::from_secs(timeout_secs),
            proxy,
            user_agent,
            max_retries,
        }
    }
}

/// Default UA: a plain `kuukan/<version>`.
pub fn default_user_agent() -> String {
    format!("kuukan/{}", env!("CARGO_PKG_VERSION"))
}

/// Async MAL client.
pub struct MalClient {
    http: reqwest::Client,
    config: MalConfig,
}

impl MalClient {
    /// Build a client from a [`MalConfig`].
    pub fn new(config: MalConfig) -> Result<Self, MalError> {
        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent.clone());
        if let Some(proxy) = &config.proxy {
            let proxy =
                reqwest::Proxy::all(proxy).map_err(|e| MalError::Transport(e.to_string()))?;
            builder = builder.proxy(proxy);
        }
        let http = builder
            .build()
            .map_err(|e| MalError::Transport(e.to_string()))?;
        Ok(MalClient { http, config })
    }

    /// Build a client from the environment (`MalConfig::from_env`).
    pub fn from_env() -> Result<Self, MalError> {
        Self::new(MalConfig::from_env())
    }

    pub fn config(&self) -> &MalConfig {
        &self.config
    }

    /// The underlying `reqwest` client (for one-off requests).
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// GET a page and parse it with libxml2.
    pub async fn get_html(&self, url: &str) -> Result<HtmlDoc, MalError> {
        let bytes = self.get_bytes(url).await?;
        HtmlDoc::parse(&bytes).map_err(MalError::from)
    }

    /// GET a page as text (charset-aware, UTF-8 fallback).
    pub async fn get_text(&self, url: &str) -> Result<String, MalError> {
        let response = self.send(url).await?;
        response
            .text()
            .await
            .map_err(|e| MalError::Transport(e.to_string()))
    }

    /// GET a JSON document.
    pub async fn get_json(&self, url: &str) -> Result<serde_json::Value, MalError> {
        let response = self.send(url).await?;
        response
            .json()
            .await
            .map_err(|e| MalError::Json(e.to_string()))
    }

    /// GET raw response bytes.
    pub async fn get_bytes(&self, url: &str) -> Result<Bytes, MalError> {
        let response = self.send(url).await?;
        response
            .bytes()
            .await
            .map_err(|e| MalError::Transport(e.to_string()))
    }

    /// Perform the request with retries for transient failures.
    async fn send(&self, url: &str) -> Result<reqwest::Response, MalError> {
        let mut retries: u32 = 0;
        loop {
            match self.http.get(url).send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    if status >= 400 {
                        if should_retry_status(status) && retries < self.config.max_retries {
                            retries += 1;
                            backoff(retries).await;
                            continue;
                        }
                        return Err(MalError::BadResponse {
                            status,
                            url: url.to_string(),
                        });
                    }
                    return Ok(response);
                }
                Err(err) => {
                    if retries < self.config.max_retries && is_retryable_transport(&err) {
                        retries += 1;
                        backoff(retries).await;
                        continue;
                    }
                    if err.is_timeout() {
                        return Err(MalError::Timeout {
                            seconds: self.config.timeout.as_secs(),
                            message: err.to_string(),
                        });
                    }
                    return Err(MalError::Transport(err.to_string()));
                }
            }
        }
    }
}

fn should_retry_status(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

fn is_retryable_transport(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request() || err.is_body()
}

/// Small exponential backoff: 100ms, 200ms, 400ms, ... capped at 2s.
async fn backoff(retry: u32) {
    let shift = retry.saturating_sub(1).min(5);
    let millis = (100u64 << shift).min(2000);
    tokio::time::sleep(Duration::from_millis(millis)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config() -> MalConfig {
        MalConfig {
            timeout: Duration::from_secs(5),
            proxy: None,
            user_agent: "kuukan-test/0".to_string(),
            max_retries: 2,
        }
    }

    #[test]
    fn default_user_agent_is_plain() {
        let ua = default_user_agent();
        assert!(ua.starts_with("kuukan/"), "{ua}");
        assert_eq!(MalConfig::default().user_agent, ua);
        assert_eq!(MalConfig::default().timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn get_html_retries_transient_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/page"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw("<html><body><p>ok</p></body></html>", "text/html"),
            )
            .mount(&server)
            .await;

        let client = MalClient::new(test_config()).unwrap();
        let doc = client
            .get_html(&format!("{}/page", server.uri()))
            .await
            .expect("retries succeed");
        assert_eq!(doc.text("//p").unwrap().as_deref(), Some("ok"));

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3);
    }

    #[tokio::test]
    async fn get_html_does_not_retry_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = MalClient::new(test_config()).unwrap();
        let err = client
            .get_html(&format!("{}/missing", server.uri()))
            .await
            .expect_err("404 is an error");
        match err {
            MalError::BadResponse { status, url } => {
                assert_eq!(status, 404);
                assert!(url.ends_with("/missing"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_html_gives_up_after_max_retries() {
        let mut config = test_config();
        config.max_retries = 1;
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/down"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = MalClient::new(config).unwrap();
        let err = client
            .get_html(&format!("{}/down", server.uri()))
            .await
            .expect_err("500 stays an error");
        assert!(matches!(err, MalError::BadResponse { status: 500, .. }));
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_json_and_text() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/list.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(r#"[{"anime_id": 1, "score": 8}]"#, "application/json"),
            )
            .mount(&server)
            .await;

        let client = MalClient::new(test_config()).unwrap();
        let json = client
            .get_json(&format!("{}/list.json", server.uri()))
            .await
            .unwrap();
        assert_eq!(json[0]["anime_id"], 1);
        let text = client
            .get_text(&format!("{}/list.json", server.uri()))
            .await
            .unwrap();
        assert!(text.contains("anime_id"));
    }

    #[tokio::test]
    async fn get_json_maps_decode_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bad.json"))
            .respond_with(ResponseTemplate::new(200).set_body_raw("not json", "application/json"))
            .mount(&server)
            .await;

        let client = MalClient::new(test_config()).unwrap();
        let err = client
            .get_json(&format!("{}/bad.json", server.uri()))
            .await
            .expect_err("decode error");
        assert!(matches!(err, MalError::Json(_)));
    }

    #[tokio::test]
    async fn transport_failure_is_transport_error() {
        let client = MalClient::new(MalConfig {
            max_retries: 0,
            ..test_config()
        })
        .unwrap();
        // Port 1 on localhost is guaranteed to refuse connections.
        let err = client
            .get_html("http://127.0.0.1:1/")
            .await
            .expect_err("connection refused");
        assert!(matches!(err, MalError::Transport(_)), "{err:?}");
    }
}
