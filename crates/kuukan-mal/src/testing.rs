//! The in-memory [`MalSource`] adapter used by tests.
//!
//! Gated behind the `testing` feature so it never reaches a release build.
//! Its whole job is to be the second adapter at the [`MalSource`] seam: with
//! it, an endpoint can be driven end to end without a network, and a parser
//! can be pinned against a captured page.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;

use crate::error::MalError;
use crate::source::MalSource;

/// A [`MalSource`] that replays recorded responses.
///
/// Unknown URLs produce `MalError::BadResponse { status: 404 }`, which is what
/// MAL returns for a missing page, so the 404-to-empty-model paths in
/// `kuukan_mal::api` are exercised rather than bypassed.
#[derive(Debug, Default, Clone)]
pub struct RecordedSource {
    pages: HashMap<String, Bytes>,
    /// Every URL asked for, in order, including ones that missed.
    requests: Arc<Mutex<Vec<String>>>,
}

impl RecordedSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a response body for an exact URL.
    pub fn insert(mut self, url: impl Into<String>, body: impl Into<Bytes>) -> Self {
        self.pages.insert(url.into(), body.into());
        self
    }

    /// Record an HTML page.
    pub fn html(self, url: impl Into<String>, html: impl AsRef<str>) -> Self {
        let body = Bytes::copy_from_slice(html.as_ref().as_bytes());
        self.insert(url, body)
    }

    /// Record a JSON document.
    pub fn json(self, url: impl Into<String>, value: &serde_json::Value) -> Self {
        let body = Bytes::from(value.to_string());
        self.insert(url, body)
    }

    /// Every URL requested so far, in order.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("requests mutex").clone()
    }

    /// How many times `url` was requested.
    pub fn hits(&self, url: &str) -> usize {
        self.requests().iter().filter(|seen| *seen == url).count()
    }
}

#[async_trait]
impl MalSource for RecordedSource {
    async fn get_bytes(&self, url: &str) -> Result<Bytes, MalError> {
        self.requests
            .lock()
            .expect("requests mutex")
            .push(url.to_string());
        match self.pages.get(url) {
            Some(body) => Ok(body.clone()),
            None => Err(MalError::BadResponse {
                status: 404,
                url: url.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MalSourceExt;

    #[tokio::test]
    async fn replays_recorded_pages_and_404s_the_rest() {
        let source = RecordedSource::new().html("https://example/1", "<html><b>hi</b></html>");

        let doc = source.get_html("https://example/1").await.expect("html");
        assert!(doc.first("//b").expect("xpath").is_some());

        let missing = source.get_bytes("https://example/2").await;
        assert!(matches!(
            missing,
            Err(MalError::BadResponse { status: 404, .. })
        ));

        assert_eq!(source.hits("https://example/1"), 1);
        assert_eq!(source.requests().len(), 2);
    }

    #[tokio::test]
    async fn json_decodes_from_the_same_bytes() {
        let source = RecordedSource::new().json("https://example/j", &serde_json::json!({"a": 1}));
        let value = source.get_json("https://example/j").await.expect("json");
        assert_eq!(value["a"], 1);
    }
}
