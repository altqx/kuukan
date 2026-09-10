//! `indexer:manga` — port of `App\Console\Commands\Indexer\MangaIndexer`.
//!
//! Uses the purarue/mal-id-cache manga list, scrapes every id with
//! `MalClient::getManga` and stores/indexes the resulting documents.

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
    common::run_media_indexer(state, Media::Manga, &options).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::common::Media;
    use kuukan_search::EntityKind as SearchKind;
    use kuukan_store::EntityKind as StoreKind;

    #[test]
    fn manga_media_maps_to_url_store_and_index() {
        assert_eq!(Media::Manga.as_str(), "manga");
        assert_eq!(
            Media::Manga.id_cache_url(),
            "https://raw.githubusercontent.com/purarue/mal-id-cache/master/cache/manga_cache.json"
        );
        assert_eq!(Media::Manga.store_kind(), StoreKind::Manga);
        assert_eq!(Media::Manga.search_kind(), SearchKind::Manga);
    }
}
