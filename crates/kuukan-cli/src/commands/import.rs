//! `kuukan import` - load a JSON dump of Jikan payloads into SQLite and the
//! search index.
//!
//! The dump format is documented in `kuukan_store::import`:
//! `{ "anime": [ {payload}, ... ], "manga": {"<mal_id>": {payload}} }`.
//! This is the migration path for an existing Jikan deployment (export its
//! MongoDB collections to this shape, then import without re-scraping MAL).
//!
//! After the SQLite import every imported kind is streamed into the search
//! pipeline (paging through the entity table), so a freshly migrated instance
//! has searchable data without running the indexer commands.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Args;
use kuukan_api::state::AppState;
use kuukan_store::EntityKind;

use super::indexer::common::search_kind_for;

/// Page size when streaming imported entities into the search index.
const INDEX_PAGE: i64 = 500;

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// JSON dump file to import.
    pub file: PathBuf,
}

pub async fn run(args: ImportArgs) -> anyhow::Result<()> {
    let state = AppState::from_env().await?;
    let report = state.store.import_json_dump(&args.file).await?;
    let indexed = index_imported(&state, &report.by_kind).await?;
    println!(
        "Imported {} entities ({} skipped: no kind/mal_id), indexed {} for search",
        report.imported, report.skipped, indexed
    );
    Ok(())
}

/// Feed every imported kind into the search pipeline.
///
/// Returns the number of indexed documents. Kinds without a search index
/// (`genre_anime`, `genre_manga`, `episode`) are skipped, like the HTTP
/// layer's scrape-on-demand indexing.
pub async fn index_imported(
    state: &AppState,
    by_kind: &BTreeMap<String, u64>,
) -> anyhow::Result<u64> {
    let mut indexed = 0u64;

    for kind_key in by_kind.keys() {
        let Ok(kind) = kind_key.parse::<EntityKind>() else {
            tracing::warn!(kind = %kind_key, "unknown entity kind in import report");
            continue;
        };
        let Some(search_kind) = search_kind_for(kind) else {
            tracing::info!(kind = %kind_key, "imported kind has no search index; skipped");
            continue;
        };

        let total = state.store.entity_count(kind).await?;
        let mut offset = 0i64;
        while offset < total {
            let page = state.store.list_entities(kind, offset, INDEX_PAGE).await?;
            if page.is_empty() {
                break;
            }
            for entity in &page {
                // Documents whose id came from the dump's object key may lack a
                // `mal_id` field; the search schema needs it, so index a copy
                // with the resolved id.
                let payload = match entity.payload.get("mal_id") {
                    Some(_) => std::borrow::Cow::Borrowed(&entity.payload),
                    None => {
                        let mut payload = entity.payload.clone();
                        if let serde_json::Value::Object(map) = &mut payload {
                            map.insert("mal_id".to_string(), serde_json::json!(entity.mal_id));
                        }
                        std::borrow::Cow::Owned(payload)
                    }
                };
                state.pipeline.index_payload(search_kind, &payload)?;
                indexed += 1;
            }
            offset += page.len() as i64;
        }

        tracing::info!(kind = %kind_key, count = total, "imported kind indexed for search");
    }

    state.pipeline.flush()?;
    Ok(indexed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::indexer::common::test_support;
    use kuukan_search::EntityKind as SearchKind;
    use serde_json::json;

    #[tokio::test]
    async fn import_indexes_searchable_kinds_only() {
        let (state, _dir) = test_support::state("import").await;
        let report = state
            .store
            .import_json_value(json!({
                "anime": [
                    {"mal_id": 1, "title": "A"},
                    {"mal_id": 2, "title": "B"}
                ],
                // "manga" is keyed: the mal_id comes from the map key, not
                // from the document.
                "manga": {"3": {"title": "C"}},
                "genres_anime": [{"mal_id": 1, "name": "Action"}],
                "episode": [{"mal_id": 9, "title": "Episode"}]
            }))
            .await
            .unwrap();
        assert_eq!(report.imported, 5);

        let indexed = index_imported(&state, &report.by_kind).await.unwrap();

        assert_eq!(indexed, 3, "genres and episodes have no search index");
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
            state
                .pipeline
                .index()
                .searcher(SearchKind::Manga)
                .unwrap()
                .num_docs(),
            1,
            "key-derived mal_id is injected before indexing"
        );
    }
}
