//! Shared indexer plumbing and the `indexer:common` command.
//!
//! This module carries the pieces every indexer shares:
//!
//! * the purarue/mal-id-cache download + merge/sort ([`parse_id_cache`]);
//! * the resumable scrape loop ([`index_ids`] / [`run_ids`]), including cursor
//!   persistence via `Store::set_indexer_cursor` and failed-id JSON files under
//!   the data directory (jikan's `storage/app/indexer/*.failed` files);
//! * small payload helpers ([`array_mal_ids`], [`schedule_mal_ids`],
//!   [`index_list`], [`sweep_removed`], [`put_cached_document`]).
//!
//! It also implements the `indexer:common` command itself: the producer and
//! magazine lists are upserted as entities and fed to the search index.

use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use kuukan_api::state::AppState;
use kuukan_core::util::{jikan_request_fingerprint, sha1_hex};
use kuukan_mal::client::MalClient;
use kuukan_mal::error::MalError;
use kuukan_search::EntityKind as SearchKind;
use kuukan_store::{EntityKind as StoreKind, StoreConfig, StoredEntity};
use serde_json::Value;

use super::IndexerOptions;

/// purarue/mal-id-cache URL for anime (mirrors `AnimeIndexer::fetchMalIds`).
pub const ANIME_ID_CACHE_URL: &str =
    "https://raw.githubusercontent.com/purarue/mal-id-cache/master/cache/anime_cache.json";
/// purarue/mal-id-cache URL for manga (mirrors `MangaIndexer::fetchMalIds`).
pub const MANGA_ID_CACHE_URL: &str =
    "https://raw.githubusercontent.com/purarue/mal-id-cache/master/cache/manga_cache.json";

/// The media kinds driven by the purarue id cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Media {
    Anime,
    Manga,
}

impl Media {
    /// `anime` / `manga`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Media::Anime => "anime",
            Media::Manga => "manga",
        }
    }

    /// The purarue/mal-id-cache URL for this media kind.
    pub const fn id_cache_url(self) -> &'static str {
        match self {
            Media::Anime => ANIME_ID_CACHE_URL,
            Media::Manga => MANGA_ID_CACHE_URL,
        }
    }

    /// Store partition for full documents of this kind.
    pub const fn store_kind(self) -> StoreKind {
        match self {
            Media::Anime => StoreKind::Anime,
            Media::Manga => StoreKind::Manga,
        }
    }

    /// Search index for this kind.
    pub const fn search_kind(self) -> SearchKind {
        match self {
            Media::Anime => SearchKind::Anime,
            Media::Manga => SearchKind::Manga,
        }
    }

    /// Scrape one entry (`MalClient::getAnime` / `getManga`).
    pub async fn fetch(self, client: &MalClient, id: i64) -> Result<Value, MalError> {
        match self {
            Media::Anime => kuukan_mal::api::anime::get_anime(client, id).await,
            Media::Manga => kuukan_mal::api::manga::get_manga(client, id).await,
        }
    }
}

/// Map a store entity kind to its search index (genres/episodes have none).
pub const fn search_kind_for(kind: StoreKind) -> Option<SearchKind> {
    match kind {
        StoreKind::Anime => Some(SearchKind::Anime),
        StoreKind::Manga => Some(SearchKind::Manga),
        StoreKind::Character => Some(SearchKind::Character),
        StoreKind::Person => Some(SearchKind::Person),
        StoreKind::User => Some(SearchKind::User),
        StoreKind::Club => Some(SearchKind::Club),
        StoreKind::Producer => Some(SearchKind::Producer),
        StoreKind::Magazine => Some(SearchKind::Magazine),
        StoreKind::GenreAnime | StoreKind::GenreManga | StoreKind::Episode => None,
    }
}

// ---------------------------------------------------------------------------
// Data directory layout
// ---------------------------------------------------------------------------

/// Directory holding indexer state (failed ids, id-cache snapshots).
///
/// Mirrors jikan's `storage/app/indexer/`; kuukan roots it at the directory of
/// the SQLite database (`KUUKAN_DB_PATH`, usually `data/`), overridable with
/// `KUUKAN_DATA_PATH`.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("KUUKAN_DATA_PATH") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let config = StoreConfig::from_env();
    match config.path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// `<data_dir>/indexer/<name>`.
pub fn indexer_path(name: &str) -> PathBuf {
    data_dir().join("indexer").join(name)
}

/// Write raw bytes, creating parent directories.
pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Write a JSON document, creating parent directories.
pub fn write_json<T: serde::Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    write_bytes(path, &serde_json::to_vec(value)?)
}

/// Load a failed-id file (`["12", ...]`), missing files yield an empty list.
pub fn load_failed_ids(path: &Path) -> Result<Vec<i64>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let ids = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing failed ids from {}", path.display()))?;
    Ok(ids)
}

/// Persist the failed-id list (jikan rewrites the file after every failure).
pub fn save_failed_ids(path: &Path, ids: &[i64]) -> Result<()> {
    write_json(path, ids)
}

// ---------------------------------------------------------------------------
// purarue/mal-id-cache
// ---------------------------------------------------------------------------

/// Fetch the raw id-cache document (also used for incremental snapshots).
pub async fn fetch_id_cache_bytes(client: &MalClient, media: Media) -> Result<Vec<u8>> {
    let url = media.id_cache_url();
    tracing::info!(url, "fetching MAL ID cache");
    let response = client
        .http()
        .get(url)
        .send()
        .await
        .with_context(|| format!("requesting {url}"))?;
    if !response.status().is_success() {
        anyhow::bail!("{url} returned HTTP {}", response.status());
    }
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("reading {url}"))?;
    Ok(bytes.to_vec())
}

/// Download and merge the purarue cache into a sorted id list.
pub async fn fetch_id_cache(client: &MalClient, media: Media) -> Result<Vec<i64>> {
    let bytes = fetch_id_cache_bytes(client, media).await?;
    parse_id_cache(&bytes)
}

/// `array_merge($ids['sfw'], $ids['nsfw']); sort($ids, SORT_NUMERIC)`.
///
/// The purarue cache is `{"sfw": [...], "nsfw": [...]}`. Ordering is numeric
/// ascending; duplicates are kept (PHP does not deduplicate either).
pub fn parse_id_cache(bytes: &[u8]) -> Result<Vec<i64>> {
    let value: Value = serde_json::from_slice(bytes).context("MAL ID cache is not valid JSON")?;
    let mut ids = Vec::new();
    for bucket in ["sfw", "nsfw"] {
        let items = value
            .get(bucket)
            .and_then(Value::as_array)
            .with_context(|| format!("MAL ID cache is missing the `{bucket}` array"))?;
        for item in items {
            let id = item
                .as_i64()
                .or_else(|| item.as_u64().and_then(|value| i64::try_from(value).ok()))
                .with_context(|| {
                    format!("MAL ID cache `{bucket}` contains a non-integer id: {item}")
                })?;
            ids.push(id);
        }
    }
    ids.sort_unstable();
    Ok(ids)
}

// ---------------------------------------------------------------------------
// Start position / progress
// ---------------------------------------------------------------------------

/// Where a run starts after applying `--index` / `--resume`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// Start (or resume) at this 0-based position in the id list.
    At {
        index: usize,
        /// `--index` pointed past the end and was reset to 0 (PHP warns).
        invalid_requested: bool,
    },
    /// The saved cursor is at/past the end of the list: nothing left to do.
    Complete,
}

/// Combine `--index` and the saved resume cursor.
///
/// jikan's indexers save the 0-based loop index after every entry and resume
/// by re-processing that entry, so a saved cursor of `n` means "restart at
/// `n`". A completed run stores `ids.len()`, which resumes as [`Start::Complete`]
/// (jikan deletes its save file at that point).
pub fn resolve_start(requested: usize, resume_cursor: Option<&str>, total: usize) -> Start {
    if let Some(cursor) = resume_cursor {
        match cursor.trim().parse::<usize>() {
            Ok(index) if index >= total => return Start::Complete,
            Ok(index) => {
                return Start::At {
                    index,
                    invalid_requested: false,
                }
            }
            Err(_) => {}
        }
    }
    if requested > 0 && requested >= total {
        return Start::At {
            index: 0,
            invalid_requested: true,
        };
    }
    Start::At {
        index: requested,
        invalid_requested: false,
    }
}

/// Outcome of a scrape loop.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IndexReport {
    /// Entries scraped, stored and indexed.
    pub indexed: usize,
    /// Entries MAL answered 404 for (jikan counts these as indexed too).
    pub not_found: usize,
    /// Ids that failed with a transport/parser/upstream error.
    pub failed: Vec<i64>,
}

impl IndexReport {
    /// Entries jikan would count as "indexed or updated" (including 404s).
    pub fn fetched(&self) -> usize {
        self.indexed + self.not_found
    }
}

/// Index `ids[start..]`, persisting progress after every entry.
///
/// `fetch` is the MAL call for one id; tests inject a closure instead of the
/// real client. Successful payloads are upserted with no TTL (`expires_at` is
/// `NULL`: the row is kept forever, the API layer applies its own TTLs), fed
/// to the search pipeline, and the cursor is advanced. The pipeline commits
/// every `IndexPipeline::DEFAULT_BATCH_SIZE` operations (250) and is flushed
/// explicitly when the run ends. A MAL 404 is skipped without a failure entry
/// (like jikan); any other error is appended to `failed_path`.
pub async fn index_ids<F, Fut>(
    state: &AppState,
    store_kind: StoreKind,
    search_kind: SearchKind,
    ids: &[i64],
    start: usize,
    delay: Duration,
    cursor_name: &str,
    failed_path: &Path,
    mut fetch: F,
) -> Result<IndexReport>
where
    F: FnMut(i64) -> Fut,
    Fut: Future<Output = std::result::Result<Value, MalError>>,
{
    let mut report = IndexReport::default();
    let mut failures: Vec<i64> = Vec::new();

    // jikan resets the save file to 0 before the loop.
    state.store.set_indexer_cursor(cursor_name, "0").await?;

    for (position, id) in ids.iter().enumerate().skip(start) {
        tracing::info!(
            kind = store_kind.as_str(),
            position = position + 1,
            total = ids.len(),
            mal_id = id,
            "indexing/updating entry"
        );

        match fetch(*id).await {
            Ok(payload) => {
                match store_and_index(state, store_kind, search_kind, *id, &payload).await {
                    Ok(()) => report.indexed += 1,
                    Err(error) => {
                        // A malformed payload or an indexing failure is a failed
                        // entry, not a fatal run error (jikan's indexer marks the
                        // same case as skipped and keeps going).
                        tracing::warn!(mal_id = id, error = %error, "skipping unindexable entry");
                        failures.push(*id);
                        save_failed_ids(failed_path, &failures)?;
                    }
                }
            }
            Err(error) if error.is_not_found() => {
                // Mirrors jikan: a 404 from the upstream endpoint is not
                // requeued, the id list simply contains entries MAL removed.
                tracing::debug!(mal_id = id, "entry no longer exists (404)");
                report.not_found += 1;
            }
            Err(error) => {
                tracing::warn!(mal_id = id, error = %error, "skipping failed entry");
                failures.push(*id);
                save_failed_ids(failed_path, &failures)?;
            }
        }

        // Persist the 0-based index after every entry so a killed run can be
        // resumed (`--resume` re-processes the last entry, like jikan).
        state
            .store
            .set_indexer_cursor(cursor_name, &position.to_string())
            .await?;

        if position + 1 < ids.len() {
            tokio::time::sleep(delay).await;
        }
    }

    state.pipeline.flush()?;

    // Mark the run as complete; `--resume` then finds nothing left to do
    // (jikan deletes its `*.save` file at the end).
    state
        .store
        .set_indexer_cursor(cursor_name, &ids.len().to_string())
        .await?;

    report.failed = failures;
    Ok(report)
}

/// Store one scraped document (TTL `None`) and feed it to its search index.
async fn store_and_index(
    state: &AppState,
    store_kind: StoreKind,
    search_kind: SearchKind,
    mal_id: i64,
    payload: &Value,
) -> Result<()> {
    let entity = StoredEntity::new(store_kind, mal_id, payload.clone());
    state.store.upsert_entity(entity, None).await?;
    state.pipeline.index_payload(search_kind, payload)?;
    Ok(())
}

/// Apply `--reverse` / `--resume` / `--index` and run [`index_ids`].
pub async fn run_ids<F, Fut>(
    state: &AppState,
    store_kind: StoreKind,
    search_kind: SearchKind,
    cursor_name: &str,
    failed_path: &Path,
    options: &IndexerOptions,
    mut ids: Vec<i64>,
    fetch: F,
) -> Result<IndexReport>
where
    F: FnMut(i64) -> Fut,
    Fut: Future<Output = std::result::Result<Value, MalError>>,
{
    // jikan reverses before reading the resume index, so a saved cursor refers
    // to the reversed list.
    if options.reverse {
        ids.reverse();
    }

    if ids.is_empty() {
        tracing::warn!(cursor = cursor_name, "no ids to index");
        return Ok(IndexReport::default());
    }

    let resume_cursor = if options.resume {
        let cursor = state.store.get_indexer_cursor(cursor_name).await?;
        if let Some(cursor) = cursor.as_deref().filter(|cursor| !cursor.is_empty()) {
            tracing::info!(cursor = cursor, "resuming from saved index");
        }
        cursor
    } else {
        None
    };

    let start = match resolve_start(options.index, resume_cursor.as_deref(), ids.len()) {
        Start::Complete => {
            tracing::info!(
                cursor = cursor_name,
                "saved cursor is complete; nothing to do"
            );
            return Ok(IndexReport::default());
        }
        Start::At {
            index,
            invalid_requested,
        } => {
            if invalid_requested {
                tracing::warn!("invalid index; set back to 0");
            }
            index
        }
    };

    tracing::info!(
        cursor = cursor_name,
        start,
        total = ids.len(),
        delay_secs = options.delay,
        "starting indexer"
    );

    index_ids(
        state,
        store_kind,
        search_kind,
        &ids,
        start,
        Duration::from_secs(options.delay),
        cursor_name,
        failed_path,
        fetch,
    )
    .await
}

/// Full media indexer behind `indexer:anime` / `indexer:manga`.
///
/// Fetches the purarue id list (or the failed ids with `--failed`), then
/// scrapes, stores and indexes every entry through [`run_ids`].
pub async fn run_media_indexer(
    state: &AppState,
    media: Media,
    options: &IndexerOptions,
) -> Result<IndexReport> {
    let cursor_name = media.as_str();
    let failed_path = indexer_path(&format!("{cursor_name}.failed.json"));

    let ids = if options.failed {
        let ids = load_failed_ids(&failed_path)?;
        if ids.is_empty() {
            tracing::warn!(
                path = %failed_path.display(),
                "no failed ids to retry"
            );
        }
        ids
    } else {
        let ids = fetch_id_cache(&state.mal, media).await?;
        // jikan caches the merged list as `indexer/<media>_mal_id.json`.
        write_json(&indexer_path(&format!("{cursor_name}_mal_id.json")), &ids)?;
        ids
    };

    let mal = state.mal.clone();
    let report = run_ids(
        state,
        media.store_kind(),
        media.search_kind(),
        cursor_name,
        &failed_path,
        options,
        ids,
        move |id| {
            let mal = mal.clone();
            async move { media.fetch(&mal, id).await }
        },
    )
    .await?;

    tracing::info!(
        media = cursor_name,
        indexed = report.fetched(),
        not_found = report.not_found,
        failed = report.failed.len(),
        "indexing complete"
    );
    Ok(report)
}

/// Refresh a set of anime documents (used by the schedule and current-season
/// indexers to keep currently-airing entries up to date).
pub async fn refresh_anime(
    state: &AppState,
    ids: Vec<i64>,
    options: &IndexerOptions,
    cursor_name: &str,
    failed_path: &Path,
) -> Result<IndexReport> {
    let mal = state.mal.clone();
    run_ids(
        state,
        StoreKind::Anime,
        SearchKind::Anime,
        cursor_name,
        failed_path,
        options,
        ids,
        move |id| {
            let mal = mal.clone();
            async move { kuukan_mal::api::anime::get_anime(&mal, id).await }
        },
    )
    .await
}

// ---------------------------------------------------------------------------
// Payload helpers
// ---------------------------------------------------------------------------

/// Collect `mal_id`s from `payload[key]` (an array of objects).
pub fn array_mal_ids(payload: &Value, key: &str) -> Vec<i64> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("mal_id").and_then(Value::as_i64))
                .collect()
        })
        .unwrap_or_default()
}

/// Collect the `mal_id`s of every entry in a schedule document.
///
/// `ScheduleParser` returns `{ monday: [entry, ...], tuesday: [...], ... }`
/// (`ScheduleController` flattens the days in the same order).
pub fn schedule_mal_ids(payload: &Value) -> Vec<i64> {
    let Some(days) = payload.as_object() else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for entries in days.values() {
        let Some(entries) = entries.as_array() else {
            continue;
        };
        for entry in entries {
            if let Some(id) = entry.get("mal_id").and_then(Value::as_i64) {
                ids.push(id);
            }
        }
    }
    ids
}

/// Upsert + index a list of entity payloads (producer/magazine lists).
pub async fn index_list(
    state: &AppState,
    store_kind: StoreKind,
    search_kind: SearchKind,
    items: &[Value],
) -> Result<usize> {
    let mut count = 0usize;
    for item in items {
        let Some(mal_id) = item.get("mal_id").and_then(Value::as_i64) else {
            tracing::warn!(kind = store_kind.as_str(), "skipping entry without mal_id");
            continue;
        };
        state
            .store
            .upsert_entity(StoredEntity::new(store_kind, mal_id, item.clone()), None)
            .await?;
        state.pipeline.index_payload(search_kind, item)?;
        count += 1;
    }
    state.pipeline.flush()?;
    Ok(count)
}

/// Store a full document under the route fingerprint (`request:<type>:<sha1>`).
pub async fn put_cached_document(
    state: &AppState,
    request_type: &str,
    uri: &str,
    ttl_secs: i64,
    payload: Value,
) -> Result<String> {
    let fingerprint = jikan_request_fingerprint(request_type, uri);
    let existing = state.store.get_cache(&fingerprint).await?.is_some();
    state
        .store
        .put_cache(&fingerprint, payload, Some(ttl_secs), existing)
        .await?;
    Ok(fingerprint)
}

/// SHA-1 of the raw id-cache snapshot (jikan compares `sha1($raw)`).
pub fn snapshot_hash(bytes: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(bytes).context("id-cache snapshot is not UTF-8")?;
    Ok(sha1_hex(text))
}

/// Delete every entity of `kind` whose `mal_id` is not in `keep_ids`, and
/// remove it from the search index.
///
/// Mirrors `AnimeSweepIndexer` / `MangaSweepIndexer`: ids that disappeared
/// from the purarue cache are deleted from the database.
pub async fn sweep_removed(
    state: &AppState,
    store_kind: StoreKind,
    search_kind: SearchKind,
    keep_ids: &HashSet<i64>,
) -> Result<usize> {
    /// Page size for the entity scan (PHP loads every id at once; paging keeps
    /// memory bounded without changing the result).
    const PAGE: i64 = 500;

    let total = state.store.entity_count(store_kind).await?;
    let mut stale: Vec<i64> = Vec::new();
    let mut offset = 0i64;
    while offset < total {
        let page = state.store.list_entities(store_kind, offset, PAGE).await?;
        if page.is_empty() {
            break;
        }
        for entity in &page {
            if !keep_ids.contains(&entity.mal_id) {
                stale.push(entity.mal_id);
            }
        }
        offset += page.len() as i64;
    }

    for mal_id in &stale {
        tracing::info!(kind = store_kind.as_str(), mal_id, "removing stale entity");
        state.store.delete_entity(store_kind, *mal_id).await?;
        if let Ok(search_id) = u64::try_from(*mal_id) {
            state.pipeline.remove(search_kind, search_id)?;
        }
    }
    state.pipeline.flush()?;

    Ok(stale.len())
}

// ---------------------------------------------------------------------------
// `indexer:common`
// ---------------------------------------------------------------------------

/// `indexer:common` — update the producer and magazine lists.
pub async fn run(options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, _options: IndexerOptions) -> Result<()> {
    let producers = kuukan_mal::api::producer::get_producers(&state.mal).await?;
    let magazines = kuukan_mal::api::magazine::get_magazines(&state.mal).await?;
    let (producers, magazines) = index_lists(state, &producers, &magazines).await?;

    tracing::info!(producers, magazines, "indexer:common complete");
    Ok(())
}

/// Upsert + index the two common lists; split out for tests without a network.
pub async fn index_lists(
    state: &AppState,
    producers_payload: &Value,
    magazines_payload: &Value,
) -> Result<(usize, usize)> {
    let producer_items = producers_payload
        .get("producers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let magazine_items = magazines_payload
        .get("magazines")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let producers = index_list(
        state,
        StoreKind::Producer,
        SearchKind::Producer,
        &producer_items,
    )
    .await?;
    let magazines = index_list(
        state,
        StoreKind::Magazine,
        SearchKind::Magazine,
        &magazine_items,
    )
    .await?;
    Ok((producers, magazines))
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use kuukan_api::config::Config;
    use kuukan_api::state::AppState;
    use kuukan_mal::client::{MalClient, MalConfig};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Unique scratch directory for one test.
    pub fn temp_dir(tag: &str) -> PathBuf {
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "kuukan-cli-indexer-{tag}-{}-{counter}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// In-memory store + scratch search index + offline MAL client.
    pub async fn state(tag: &str) -> (AppState, PathBuf) {
        let dir = temp_dir(tag);
        let store = kuukan_store::Store::open_in_memory()
            .await
            .expect("open in-memory store");
        let index = kuukan_search::SearchIndex::open(&dir).expect("open search index");
        let mal = MalClient::new(MalConfig::default()).expect("build mal client");
        let state = AppState::new(
            Config::default(),
            store,
            kuukan_search::IndexPipeline::new(index),
            mal,
        );
        (state, dir)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support;
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_id_cache_merges_and_sorts() {
        let bytes = br#"{"sfw": [5, 1, 3], "nsfw": [4, 2]}"#;
        assert_eq!(parse_id_cache(bytes).unwrap(), vec![1, 2, 3, 4, 5]);

        // Duplicates survive the merge, like PHP's array_merge + sort.
        let bytes = br#"{"sfw": [2, 1], "nsfw": [1]}"#;
        assert_eq!(parse_id_cache(bytes).unwrap(), vec![1, 1, 2]);

        assert_eq!(
            parse_id_cache(br#"{"sfw": [], "nsfw": []}"#).unwrap(),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn parse_id_cache_rejects_bad_shapes() {
        assert!(parse_id_cache(b"not json").is_err());
        assert!(parse_id_cache(br#"{"sfw": [1]}"#).is_err(), "nsfw missing");
        assert!(
            parse_id_cache(br#"{"sfw": ["1"], "nsfw": []}"#).is_err(),
            "string ids are rejected"
        );
    }

    #[test]
    fn failed_ids_round_trip_and_missing_file() {
        let dir = test_support::temp_dir("failed");
        let path = dir.join("indexer/anime.failed.json");

        assert_eq!(load_failed_ids(&path).unwrap(), Vec::<i64>::new());
        save_failed_ids(&path, &[7, 9]).unwrap();
        assert_eq!(load_failed_ids(&path).unwrap(), vec![7, 9]);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[7,9]");
    }

    #[test]
    fn resolve_start_honors_index_and_resume() {
        assert_eq!(
            resolve_start(0, None, 3),
            Start::At {
                index: 0,
                invalid_requested: false
            }
        );
        assert_eq!(
            resolve_start(2, None, 3),
            Start::At {
                index: 2,
                invalid_requested: false
            }
        );
        // Resume overrides --index and re-processes the saved entry.
        assert_eq!(
            resolve_start(2, Some("1"), 3),
            Start::At {
                index: 1,
                invalid_requested: false
            }
        );
        // An unparsable cursor falls back to --index.
        assert_eq!(
            resolve_start(1, Some("nonsense"), 3),
            Start::At {
                index: 1,
                invalid_requested: false
            }
        );
    }

    #[test]
    fn resolve_start_reports_invalid_index_and_complete_cursor() {
        assert_eq!(
            resolve_start(5, None, 3),
            Start::At {
                index: 0,
                invalid_requested: true
            }
        );
        assert_eq!(
            resolve_start(0, Some("3"), 3),
            Start::Complete,
            "the completion marker resumes as a no-op"
        );
        assert_eq!(
            resolve_start(0, Some("9"), 3),
            Start::Complete,
            "a cursor past the end is treated as complete"
        );
    }

    #[test]
    fn payload_helpers_extract_only_top_level_ids() {
        let producers = json!({
            "producers": [
                {"mal_id": 1, "name": "Studio Pierrot"},
                {"mal_id": 2, "name": "Madhouse"},
                {"name": "no id"}
            ]
        });
        assert_eq!(array_mal_ids(&producers, "producers"), vec![1, 2]);
        assert_eq!(array_mal_ids(&producers, "magazines"), Vec::<i64>::new());

        let schedule = json!({
            "monday": [{"mal_id": 10, "title": "A"}],
            "tuesday": [{"mal_id": 20}, {"mal_id": 21}],
            "wednesday": []
        });
        assert_eq!(schedule_mal_ids(&schedule), vec![10, 20, 21]);
        assert_eq!(schedule_mal_ids(&json!([])), Vec::<i64>::new());
    }

    #[tokio::test]
    async fn index_ids_upserts_and_indexes() {
        let (state, _dir) = test_support::state("index-ids").await;
        let failed_path = test_support::temp_dir("index-ids-failed").join("anime.failed.json");

        let report = index_ids(
            &state,
            StoreKind::Anime,
            SearchKind::Anime,
            &[1, 2],
            0,
            Duration::ZERO,
            "anime",
            &failed_path,
            |id| async move { Ok(json!({"mal_id": id, "title": format!("Anime {id}")})) },
        )
        .await
        .unwrap();

        assert_eq!(report.indexed, 2);
        assert_eq!(report.not_found, 0);
        assert!(report.failed.is_empty());
        assert_eq!(report.fetched(), 2);
        assert_eq!(state.store.entity_count(StoreKind::Anime).await.unwrap(), 2);
        assert!(state
            .store
            .get_entity(StoreKind::Anime, 2)
            .await
            .unwrap()
            .is_some());
        // TTL is NULL: the entity never expires in the store.
        assert_eq!(
            state
                .store
                .get_entity(StoreKind::Anime, 2)
                .await
                .unwrap()
                .unwrap()
                .expires_at,
            None
        );
        assert_eq!(
            state
                .pipeline
                .index()
                .searcher(SearchKind::Anime)
                .unwrap()
                .num_docs(),
            2
        );
        assert_eq!(
            state.store.get_indexer_cursor("anime").await.unwrap(),
            Some("2".to_string())
        );
    }

    #[tokio::test]
    async fn index_ids_records_failures_and_404s() {
        let (state, _dir) = test_support::state("index-failed").await;
        let dir = test_support::temp_dir("index-failed-ids");
        let failed_path = dir.join("anime.failed.json");

        let report = index_ids(
            &state,
            StoreKind::Anime,
            SearchKind::Anime,
            &[1, 2, 3],
            0,
            Duration::ZERO,
            "anime",
            &failed_path,
            |id| async move {
                match id {
                    2 => Err(MalError::BadResponse {
                        status: 404,
                        url: "https://myanimelist.net/anime/2".into(),
                    }),
                    3 => Err(MalError::Transport("boom".into())),
                    _ => Ok(json!({"mal_id": id})),
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(report.indexed, 1);
        assert_eq!(report.not_found, 1);
        assert_eq!(report.failed, vec![3]);
        assert_eq!(report.fetched(), 2);
        assert_eq!(state.store.entity_count(StoreKind::Anime).await.unwrap(), 1);
        assert_eq!(load_failed_ids(&failed_path).unwrap(), vec![3]);
        // Progress advanced through the failed entry as well.
        assert_eq!(
            state.store.get_indexer_cursor("anime").await.unwrap(),
            Some("3".to_string())
        );
    }

    #[tokio::test]
    async fn run_ids_reverses_and_resumes_from_cursor() {
        let (state, _dir) = test_support::state("run-ids").await;
        let failed_path = test_support::temp_dir("run-ids-failed").join("anime.failed.json");
        state.store.set_indexer_cursor("anime", "1").await.unwrap();

        let mut requested = Vec::new();
        let options = IndexerOptions {
            delay: 0,
            resume: true,
            ..IndexerOptions::default()
        };
        let report = run_ids(
            &state,
            StoreKind::Anime,
            SearchKind::Anime,
            "anime",
            &failed_path,
            &options,
            vec![1, 2, 3],
            |id| {
                requested.push(id);
                async move { Ok(json!({"mal_id": id})) }
            },
        )
        .await
        .unwrap();

        assert_eq!(requested, vec![2, 3], "resume starts at the saved index");
        assert_eq!(report.indexed, 2);

        // A completed cursor is a no-op.
        let options = IndexerOptions {
            delay: 0,
            resume: true,
            ..IndexerOptions::default()
        };
        let report = run_ids(
            &state,
            StoreKind::Anime,
            SearchKind::Anime,
            "anime",
            &failed_path,
            &options,
            vec![1, 2, 3],
            |id| async move { Ok(json!({"mal_id": id})) },
        )
        .await
        .unwrap();
        assert_eq!(report.indexed, 0);
    }

    #[tokio::test]
    async fn run_ids_reverse_uses_reversed_positions() {
        let (state, _dir) = test_support::state("run-reverse").await;
        let failed_path = test_support::temp_dir("run-reverse-failed").join("anime.failed.json");

        let mut requested = Vec::new();
        let options = IndexerOptions {
            delay: 0,
            reverse: true,
            ..IndexerOptions::default()
        };
        run_ids(
            &state,
            StoreKind::Anime,
            SearchKind::Anime,
            "anime",
            &failed_path,
            &options,
            vec![1, 2, 3],
            |id| {
                requested.push(id);
                async move { Ok(json!({"mal_id": id})) }
            },
        )
        .await
        .unwrap();

        assert_eq!(requested, vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn put_cached_document_uses_route_fingerprint_and_ttl() {
        let (state, _dir) = test_support::state("cached-doc").await;
        let payload = json!({"genres": []});

        let fingerprint = put_cached_document(
            &state,
            "genres",
            "/v1/genres/anime",
            432_000,
            payload.clone(),
        )
        .await
        .unwrap();

        assert_eq!(
            fingerprint,
            "request:genres:221ee7a02c2cb0a335ddebfa11e7d913287f29ea"
        );
        let entry = state.store.get_cache(&fingerprint).await.unwrap().unwrap();
        assert_eq!(entry.payload["genres"], payload["genres"]);
        assert!(entry.expires_at.unwrap() > entry.modified_at);
    }

    #[tokio::test]
    async fn sweep_removed_deletes_and_unindexes() {
        let (state, _dir) = test_support::state("sweep").await;

        for id in [1, 2, 3] {
            state
                .store
                .upsert_entity(
                    StoredEntity::new(StoreKind::Anime, id, json!({"mal_id": id, "title": "x"})),
                    None,
                )
                .await
                .unwrap();
            state
                .pipeline
                .index_payload(SearchKind::Anime, &json!({"mal_id": id, "title": "x"}))
                .unwrap();
        }
        state.pipeline.flush().unwrap();

        let keep: HashSet<i64> = [1, 3].into_iter().collect();
        let removed = sweep_removed(&state, StoreKind::Anime, SearchKind::Anime, &keep)
            .await
            .unwrap();

        assert_eq!(removed, 1);
        assert_eq!(state.store.entity_count(StoreKind::Anime).await.unwrap(), 2);
        assert!(state
            .store
            .get_entity(StoreKind::Anime, 2)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            state
                .pipeline
                .index()
                .searcher(SearchKind::Anime)
                .unwrap()
                .num_docs(),
            2
        );
    }

    #[tokio::test]
    async fn index_lists_upserts_producers_and_magazines() {
        let (state, _dir) = test_support::state("common-lists").await;

        let (producers, magazines) = index_lists(
            &state,
            &json!({"producers": [{"mal_id": 1, "name": "Pierrot", "url": "u", "count": 3}]}),
            &json!({"magazines": [
                {"mal_id": 2, "name": "Big Comic", "url": "u", "count": 4},
                {"name": "missing id"}
            ]}),
        )
        .await
        .unwrap();

        assert_eq!((producers, magazines), (1, 1));
        assert!(state
            .store
            .get_entity(StoreKind::Producer, 1)
            .await
            .unwrap()
            .is_some());
        assert!(state
            .store
            .get_entity(StoreKind::Magazine, 2)
            .await
            .unwrap()
            .is_some());
        assert_eq!(
            state
                .pipeline
                .index()
                .searcher(SearchKind::Producer)
                .unwrap()
                .num_docs(),
            1
        );
        assert_eq!(
            state
                .pipeline
                .index()
                .searcher(SearchKind::Magazine)
                .unwrap()
                .num_docs(),
            1
        );
    }
}
