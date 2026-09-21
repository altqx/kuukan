//! End-to-end tests over the real router.
//!
//! These drive `build_router` exactly as `serve()` does, with a recorded
//! [`MalSource`] in place of the HTTP one. They cover the sequencing that the
//! per-module unit tests cannot reach: whether a fresh cache document is
//! served without scraping, whether a scrape is persisted, whether the
//! fingerprint in the response header is the one the cache is keyed on, and
//! how an upstream 404 reaches the client.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use kuukan_api::config::Config;
use kuukan_api::server::build_router;
use kuukan_api::state::AppState;
use kuukan_mal::testing::RecordedSource;
use kuukan_search::{IndexPipeline, SearchIndex};
use kuukan_store::{EntityKind, Store, StoreConfig, StoredEntity};
use serde_json::Value;
use tower::ServiceExt;

/// A router plus the pieces a test needs to look behind it.
struct Harness {
    state: AppState,
    source: RecordedSource,
    _dir: tempfile::TempDir,
}

impl Harness {
    async fn new(source: RecordedSource) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(StoreConfig::new(dir.path().join("kuukan.db")))
            .await
            .expect("open store");
        let index = SearchIndex::open(dir.path().join("search")).expect("open search index");
        let state = AppState::new(
            Config::default(),
            store,
            IndexPipeline::new(index),
            source.clone(),
        );
        Harness {
            state,
            source,
            _dir: dir,
        }
    }

    async fn get(&self, uri: &str) -> (StatusCode, Vec<(String, String)>, Value) {
        let response = build_router(self.state.clone())
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let bytes = to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .expect("body");
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, headers, json)
    }
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// A fresh stored entity is served straight from the store: the source is
/// never asked, and the Jikan cache flags describe the stored document.
#[tokio::test]
async fn fresh_entity_is_served_without_scraping() {
    let harness = Harness::new(RecordedSource::new()).await;
    harness
        .state
        .store
        .upsert_entity(
            StoredEntity::new(
                EntityKind::Anime,
                1,
                serde_json::json!({ "mal_id": 1, "title": "Cowboy Bebop" }),
            ),
            Some(86_400),
        )
        .await
        .expect("seed entity");

    let (status, headers, body) = harness.get("/v1/anime/1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["mal_id"], 1);
    assert_eq!(body["data"]["title"], "Cowboy Bebop");
    assert!(
        harness.source.requests().is_empty(),
        "a fresh entity must not reach the source, asked for {:?}",
        harness.source.requests()
    );
    assert_eq!(
        header(&headers, "cache-control"),
        Some("public, s-maxage=86400")
    );
    assert!(header(&headers, "last-modified").is_some());
}

/// The `X-Request-Fingerprint` header must be the key the cache document is
/// stored under. The two are derived at different call sites, so nothing but a
/// test through the router can catch them drifting apart.
#[tokio::test]
async fn response_fingerprint_is_the_cache_key() {
    // `/anime/{id}/episodes` answers an upstream 404 with an empty model, so
    // this exercises a real scrape-and-persist without needing a captured page.
    let harness = Harness::new(RecordedSource::new()).await;

    let (status, headers, body) = harness.get("/v1/anime/1/episodes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], serde_json::json!([]));

    let fingerprint = header(&headers, "x-request-fingerprint").expect("fingerprint header");
    let stored = harness
        .state
        .store
        .get_cache(fingerprint)
        .await
        .expect("cache lookup");
    assert!(
        stored.is_some(),
        "no cache document is stored under the advertised fingerprint {fingerprint}"
    );
}

/// A scraped response is persisted, so the second request is served from the
/// cache and the source is asked exactly once.
#[tokio::test]
async fn a_scrape_is_persisted_and_then_reused() {
    let harness = Harness::new(RecordedSource::new()).await;

    let (first, _, _) = harness.get("/v1/anime/1/episodes").await;
    let asked_after_first = harness.source.requests().len();
    let (second, _, _) = harness.get("/v1/anime/1/episodes").await;

    assert_eq!(first, StatusCode::OK);
    assert_eq!(second, StatusCode::OK);
    assert_eq!(asked_after_first, 1, "the first request must scrape once");
    assert_eq!(
        harness.source.requests().len(),
        1,
        "the second request must be served from the cache"
    );
}

/// An upstream 404 on a lookup endpoint reaches the client as Jikan's 404
/// envelope, not as a 500.
#[tokio::test]
async fn upstream_404_becomes_the_jikan_404_envelope() {
    let harness = Harness::new(RecordedSource::new()).await;

    let (status, _, body) = harness.get("/v1/anime/999999").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
    assert_eq!(body["type"], "BadResponseException");
    assert_eq!(
        harness.source.requests(),
        vec!["https://myanimelist.net/anime/999999/".to_string()],
        "the lookup must reach the source exactly once, at the upstream URL"
    );
}

/// A non-numeric id never matches the Laravel route constraint, so it is a
/// plain 404 and must not reach the source at all.
#[tokio::test]
async fn non_numeric_id_is_a_route_miss() {
    let harness = Harness::new(RecordedSource::new()).await;

    let (status, _, body) = harness.get("/v1/anime/not-a-number").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "HttpException");
    assert!(harness.source.requests().is_empty());
}

/// An unknown path falls back to the Jikan 404 envelope.
#[tokio::test]
async fn unknown_path_falls_back_to_404() {
    let harness = Harness::new(RecordedSource::new()).await;

    let (status, _, body) = harness.get("/v1/nope").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404);
}

/// The metadata document at `/` reads source health out of the store.
#[tokio::test]
async fn root_serves_the_metadata_document() {
    let harness = Harness::new(RecordedSource::new()).await;

    let (status, _, body) = harness.get("/").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["version"], "4.2.2");
    assert!(body["myanimelist_heartbeat"]["status"].is_string());
}

/// Query parameters that fail DTO validation are rejected before any scrape.
#[tokio::test]
async fn invalid_query_is_rejected_before_scraping() {
    let harness = Harness::new(RecordedSource::new()).await;

    let (status, _, _) = harness.get("/v1/anime/1/episodes?page=notanumber").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        harness.source.requests().is_empty(),
        "validation must run before the source is touched"
    );
}

/// Genre lists are media-wide documents, so `?page=` must not split the cache.
/// Seeding the document under the path-only key and asking for page 2 proves
/// the endpoint keys on the path alone, end to end.
#[tokio::test]
async fn genre_lists_ignore_pagination_in_the_cache_key() {
    let harness = Harness::new(RecordedSource::new()).await;
    let key = kuukan_api::endpoint::fingerprint("genres", "/v1/genres/anime");
    harness
        .state
        .store
        .put_cache(
            &key,
            serde_json::json!({ "genres": [{ "mal_id": 1, "name": "Action" }] }),
            Some(432_000),
            false,
        )
        .await
        .expect("seed cache document");

    let (status, _, body) = harness.get("/v1/genres/anime?page=2").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"][0]["name"], "Action");
    assert!(
        harness.source.requests().is_empty(),
        "a paginated genre list must reuse the media-wide document"
    );
}

/// Profile endpoints deliberately key the cache on the username alone while
/// the header still hashes the request that was made. Both halves of that
/// divergence are load-bearing, so pin them together.
#[tokio::test]
async fn profile_endpoints_key_on_username_but_fingerprint_the_request() {
    let harness = Harness::new(RecordedSource::new()).await;
    let key = kuukan_api::endpoint::fingerprint("users", "/v1/users/someone");
    harness
        .state
        .store
        .put_cache(
            &key,
            serde_json::json!({ "username": "someone" }),
            Some(300),
            false,
        )
        .await
        .expect("seed cache document");

    let (status, headers, _) = harness.get("/v1/users/someone?page=2").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        harness.source.requests().is_empty(),
        "the username-keyed document must be reused whatever the query"
    );
    assert_eq!(
        header(&headers, "x-request-fingerprint"),
        Some(kuukan_api::endpoint::fingerprint("users", "/v1/users/someone?page=2").as_str()),
        "the header hashes the request URI, not the cache key"
    );
}
