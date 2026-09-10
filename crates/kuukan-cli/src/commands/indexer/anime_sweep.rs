//! `indexer:anime-sweep` — port of `App\Console\Commands\Indexer\AnimeSweepIndexer`.
//!
//! Fetches the purarue anime id list and deletes every stored anime that is no
//! longer in it (MAL removed the entry), mirroring the PHP sweeper.

use std::collections::HashSet;

use anyhow::Result;
use kuukan_api::state::AppState;
use kuukan_search::EntityKind as SearchKind;
use kuukan_store::EntityKind as StoreKind;

use super::common::{self, Media};
use super::IndexerOptions;

pub async fn run(_options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, _options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, _options: IndexerOptions) -> Result<()> {
    let ids = common::fetch_id_cache(&state.mal, Media::Anime).await?;
    let removed = sweep_against(state, &ids).await?;
    tracing::info!(removed, "anime sweep complete");
    Ok(())
}

/// Delete stored anime ids missing from `ids` (split out for tests).
pub(crate) async fn sweep_against(state: &AppState, ids: &[i64]) -> Result<usize> {
    let keep: HashSet<i64> = ids.iter().copied().collect();
    common::sweep_removed(state, StoreKind::Anime, SearchKind::Anime, &keep).await
}

#[cfg(test)]
mod tests {
    use super::super::common::test_support;
    use super::*;
    use kuukan_store::StoredEntity;
    use serde_json::json;

    #[tokio::test]
    async fn sweep_deletes_ids_removed_from_the_cache() {
        let (state, _dir) = test_support::state("anime-sweep").await;
        for id in [1, 2, 3] {
            state
                .store
                .upsert_entity(
                    StoredEntity::new(StoreKind::Anime, id, json!({"mal_id": id})),
                    None,
                )
                .await
                .unwrap();
        }

        let removed = sweep_against(&state, &[1, 3]).await.unwrap();

        assert_eq!(removed, 1);
        assert!(state
            .store
            .get_entity(StoreKind::Anime, 2)
            .await
            .unwrap()
            .is_none());
        assert_eq!(state.store.entity_count(StoreKind::Anime).await.unwrap(), 2);
    }
}
