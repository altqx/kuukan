//! `indexer:producers` — port of `App\Console\Commands\Indexer\ProducersIndexer`.
//!
//! Scrapes the full producer list for ids, then fetches each producer page
//! (`MalClient::getProducer`) and stores/indexes the complete document, so the
//! `/producers/{id}` endpoints can serve it from the entity table.

use anyhow::Result;
use kuukan_api::state::AppState;
use kuukan_search::EntityKind as SearchKind;
use kuukan_store::EntityKind as StoreKind;

use super::common;
use super::IndexerOptions;

pub async fn run(options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, options: IndexerOptions) -> Result<()> {
    let failed_path = common::indexer_path("producers.failed.json");

    let ids = if options.failed {
        let ids = common::load_failed_ids(&failed_path)?;
        if ids.is_empty() {
            tracing::warn!(path = %failed_path.display(), "no failed ids to retry");
        }
        ids
    } else {
        // `ProducersIndexer::fetchMalIds` scrapes the producer list page.
        let payload = kuukan_mal::api::producer::get_producers(&state.mal).await?;
        let ids = common::array_mal_ids(&payload, "producers");
        common::write_json(&common::indexer_path("producers_mal_id.json"), &ids)?;
        ids
    };

    let mal = state.mal.clone();
    let report = common::run_ids(
        state,
        StoreKind::Producer,
        SearchKind::Producer,
        "producers",
        &failed_path,
        &options,
        ids,
        move |id| {
            let mal = mal.clone();
            async move { kuukan_mal::api::producer::get_producer(&mal, id, 1).await }
        },
    )
    .await?;

    tracing::info!(
        indexed = report.fetched(),
        not_found = report.not_found,
        failed = report.failed.len(),
        "producers indexing complete"
    );
    Ok(())
}
