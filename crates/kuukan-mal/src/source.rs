//! The seam between kuukan and MyAnimeList.
//!
//! Everything kuukan needs from MAL is "give me the bytes at this URL".
//! [`MalSource`] is that one method; [`MalClient`](crate::client::MalClient)
//! is the adapter that fetches them over HTTP, and
//! [`RecordedSource`](crate::testing::RecordedSource) is the adapter that
//! replays captured pages in tests.
//!
//! Decoding sits on this side of the seam, not behind it: [`MalSourceExt`]
//! turns bytes into an [`HtmlDoc`] or a JSON [`Value`], so an adapter never
//! has to reimplement parsing and `HtmlDoc` — which is not `Send` — never has
//! to cross a trait object.

use std::future::Future;

use async_trait::async_trait;
use bytes::Bytes;
use serde_json::Value;

use crate::error::MalError;
use crate::parser::helper::HtmlDoc;

/// A source of MyAnimeList pages.
///
/// One method, because one is all the callers need. Implementors own
/// transport concerns entirely: timeouts, proxying, retries and the mapping of
/// an HTTP status `>= 400` onto [`MalError::BadResponse`].
#[async_trait]
pub trait MalSource: Send + Sync {
    /// Fetch the raw bytes at `url`.
    ///
    /// A status `>= 400` is an error, not an empty body: return
    /// [`MalError::BadResponse`] carrying the status and URL, because callers
    /// branch on 404 to produce empty models.
    async fn get_bytes(&self, url: &str) -> Result<Bytes, MalError>;
}

/// Decoding helpers available on every [`MalSource`].
///
/// These are provided methods rather than trait requirements so that adding a
/// new adapter stays a one-method job.
pub trait MalSourceExt: MalSource {
    /// Fetch a page and parse it with libxml2.
    fn get_html(&self, url: &str) -> impl Future<Output = Result<HtmlDoc, MalError>> + Send {
        async move {
            let bytes = self.get_bytes(url).await?;
            HtmlDoc::parse(&bytes).map_err(MalError::from)
        }
    }

    /// Fetch a JSON document.
    fn get_json(&self, url: &str) -> impl Future<Output = Result<Value, MalError>> + Send {
        async move {
            let bytes = self.get_bytes(url).await?;
            serde_json::from_slice(&bytes).map_err(|err| MalError::Json(err.to_string()))
        }
    }
}

impl<T: MalSource + ?Sized> MalSourceExt for T {}

/// Shared handles are sources too, so a handler can pass its
/// `Arc<dyn MalSource>` straight through without reaching inside it.
#[async_trait]
impl<T: MalSource + ?Sized> MalSource for std::sync::Arc<T> {
    async fn get_bytes(&self, url: &str) -> Result<Bytes, MalError> {
        (**self).get_bytes(url).await
    }
}
