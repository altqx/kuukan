//! `indexer:anime` — port of `App\Console\Commands\Indexer\AnimeIndexer`.
//!
//! Uses the purarue/mal-id-cache anime list, scrapes every id with
//! `MalClient::getAnime` and stores/indexes the resulting documents.

use anyhow::Result;
use kuukan_api::state::AppState;

use super::common::{self, Media};
use super::IndexerOptions;

pub async fn run(options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, options: IndexerOptions) -> Result<()> {
    common::run_media_indexer(state, Media::Anime, &options).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::common::Media;
    use kuukan_search::EntityKind as SearchKind;
    use kuukan_store::EntityKind as StoreKind;

    #[test]
    fn anime_media_maps_to_url_store_and_index() {
        assert_eq!(Media::Anime.as_str(), "anime");
        assert_eq!(
            Media::Anime.id_cache_url(),
            "https://raw.githubusercontent.com/purarue/mal-id-cache/master/cache/anime_cache.json"
        );
        assert_eq!(Media::Anime.store_kind(), StoreKind::Anime);
        assert_eq!(Media::Anime.search_kind(), SearchKind::Anime);
    }
}
