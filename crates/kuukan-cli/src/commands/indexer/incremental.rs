//! `indexer:incremental` — port of
//! `App\Console\Commands\Indexer\IncrementalIndexer`.
//!
//! Keeps a raw snapshot of the purarue/mal-id-cache document under
//! `<data_dir>/indexer/incremental/<media>.json`. When the cache changed since
//! the last run, the added ids are scraped/indexed and the fresh snapshot is
//! finalized.
//!
//! Upstream computes `array_diff($existing, $new)` (the ids *removed* from the
//! cache) and then requests them from its own API, where they 404 and nothing
//! is indexed. Kuukan follows the documented intent — compare the updated cache
//! and refresh the changed entries — and indexes ids that are new in the
//! cache; ids that disappeared are handled by the sweep indexers.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use kuukan_api::state::AppState;
use serde_json::Value;

use super::common::{self, Media};
use super::{IncrementalOptions, IndexerOptions};

pub async fn run(options: IncrementalOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state.
pub async fn run_with_state(state: &AppState, options: IncrementalOptions) -> Result<()> {
    validate(&options)?;
    for media in &options.media {
        run_media(state, *media, &options).await?;
    }
    Ok(())
}

/// `--resume` and `--failed` are mutually exclusive (PHP validator).
pub fn validate(options: &IncrementalOptions) -> Result<()> {
    if options.resume && options.failed {
        anyhow::bail!("--resume and --failed are mutually exclusive");
    }
    if options.media.is_empty() {
        anyhow::bail!("provide at least one media type: anime, manga");
    }
    Ok(())
}

async fn run_media(state: &AppState, media: Media, options: &IncrementalOptions) -> Result<()> {
    let snapshot_path = common::indexer_path(&format!("incremental/{}.json", media.as_str()));
    let failed_path = common::indexer_path(&format!("incremental/{}.failed.json", media.as_str()));
    let cursor_name = format!("incremental:{}", media.as_str());

    // The new snapshot is staged until the run succeeded, then renamed over
    // the previous one (PHP writes `<media>.json.tmp` and moves it at the end).
    let mut staged: Option<(PathBuf, PathBuf)> = None;

    let ids = if options.failed {
        let ids = common::load_failed_ids(&failed_path)?;
        if ids.is_empty() {
            tracing::warn!(path = %failed_path.display(), "no failed ids to retry");
        }
        ids
    } else {
        let raw = common::fetch_id_cache_bytes(&state.mal, media).await?;
        let previous = read_snapshot(&snapshot_path)?;
        if let Some(previous) = &previous {
            if common::snapshot_hash(previous)? == common::snapshot_hash(&raw)? {
                tracing::info!(
                    media = media.as_str(),
                    "MAL ID cache unchanged; nothing to do"
                );
                return Ok(());
            }
        }

        let ids = match previous {
            Some(previous) => {
                let previous: Value = serde_json::from_slice(&previous)
                    .context("stored MAL ID cache snapshot is not valid JSON")?;
                let current: Value = serde_json::from_slice(&raw)
                    .context("downloaded MAL ID cache is not valid JSON")?;
                diff_added_ids(&previous, &current)
            }
            None => common::parse_id_cache(&raw)?,
        };

        let tmp_path = snapshot_path.with_extension("json.tmp");
        common::write_bytes(&tmp_path, &raw)?;
        staged = Some((tmp_path, snapshot_path.clone()));
        ids
    };

    if ids.is_empty() {
        tracing::info!(media = media.as_str(), "no changed entries to index");
    } else {
        let indexer_options = IndexerOptions {
            delay: options.delay,
            index: 0,
            reverse: false,
            resume: options.resume,
            failed: false,
        };
        let mal = state.mal.clone();
        let report = common::run_ids(
            state,
            media.store_kind(),
            media.search_kind(),
            &cursor_name,
            &failed_path,
            &indexer_options,
            ids,
            move |id| {
                let mal = mal.clone();
                async move { media.fetch(&mal, id).await }
            },
        )
        .await?;
        tracing::info!(
            media = media.as_str(),
            indexed = report.indexed,
            not_found = report.not_found,
            failed = report.failed.len(),
            "incremental indexing complete"
        );
    }

    if let Some((tmp, final_path)) = staged {
        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::rename(&tmp, &final_path)
            .with_context(|| format!("finalizing {}", final_path.display()))?;
    }
    Ok(())
}

/// Read a raw id-cache snapshot, if one exists.
fn read_snapshot(path: &Path) -> Result<Option<Vec<u8>>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(Some(bytes))
}

/// Ids present in `current` but not in `previous`.
///
/// See the module docs for how this relates to upstream's `array_diff`.
pub fn diff_added_ids(previous: &Value, current: &Value) -> Vec<i64> {
    let previous_ids: HashSet<i64> = id_buckets(previous).into_iter().collect();
    let mut ids: Vec<i64> = id_buckets(current)
        .into_iter()
        .filter(|id| !previous_ids.contains(id))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Merge the `sfw` and `nsfw` arrays of an id-cache document.
fn id_buckets(value: &Value) -> Vec<i64> {
    let mut ids = Vec::new();
    for bucket in ["sfw", "nsfw"] {
        let Some(items) = value.get(bucket).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            if let Some(id) = item
                .as_i64()
                .or_else(|| item.as_u64().and_then(|value| i64::try_from(value).ok()))
            {
                ids.push(id);
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::super::common::test_support;
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_rejects_resume_with_failed() {
        let options = IncrementalOptions {
            media: vec![Media::Anime],
            delay: 3,
            resume: true,
            failed: true,
        };
        assert!(validate(&options).is_err());
        assert!(validate(&IncrementalOptions {
            failed: false,
            ..options
        })
        .is_ok());
    }

    #[test]
    fn diff_added_ids_returns_new_entries_in_order() {
        let previous = json!({"sfw": [1, 3, 5], "nsfw": [7]});
        let current = json!({"sfw": [1, 3, 4, 5], "nsfw": [7, 9]});
        assert_eq!(diff_added_ids(&previous, &current), vec![4, 9]);

        // A first run (no snapshot) is handled by the caller; an empty cache
        // diff yields nothing.
        assert_eq!(diff_added_ids(&current, &current), Vec::<i64>::new());
        assert_eq!(diff_added_ids(&json!({}), &current), vec![1, 3, 4, 5, 7, 9]);
    }

    #[test]
    fn snapshot_round_trip_and_hash() {
        let dir = test_support::temp_dir("incremental");
        let path = dir.join("incremental/anime.json");
        assert_eq!(read_snapshot(&path).unwrap(), None);

        let raw = br#"{"sfw": [1, 2], "nsfw": []}"#;
        common::write_bytes(&path, raw).unwrap();
        assert_eq!(read_snapshot(&path).unwrap().unwrap(), raw);
        assert_eq!(
            common::snapshot_hash(raw).unwrap(),
            common::snapshot_hash(&read_snapshot(&path).unwrap().unwrap()).unwrap()
        );
        assert_ne!(
            common::snapshot_hash(raw).unwrap(),
            common::snapshot_hash(br#"{"sfw": [1], "nsfw": []}"#).unwrap()
        );
    }
}
