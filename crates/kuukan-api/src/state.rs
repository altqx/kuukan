//! Shared application state for the HTTP layer.

use crate::config::Config;
use kuukan_mal::client::{MalClient, MalConfig};
use kuukan_search::{IndexPipeline, SearchIndex};
use kuukan_store::Store;
use std::sync::Arc;

/// Errors while building application state.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("storage error: {0}")]
    Store(#[from] kuukan_store::StoreError),
    #[error("search index error: {0}")]
    Search(#[from] kuukan_search::SearchError),
    #[error("mal client error: {0}")]
    Mal(#[from] kuukan_mal::error::MalError),
}

/// State available to every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub store: Arc<Store>,
    /// Search indexing pipeline (also gives access to the search index).
    pub pipeline: Arc<IndexPipeline>,
    /// MyAnimeList HTTP client.
    pub mal: Arc<MalClient>,
}

impl AppState {
    pub fn new(config: Config, store: Store, pipeline: IndexPipeline, mal: MalClient) -> Self {
        AppState {
            config: Arc::new(config),
            store: Arc::new(store),
            pipeline: Arc::new(pipeline),
            mal: Arc::new(mal),
        }
    }

    /// The shared search index.
    pub fn search_index(&self) -> &SearchIndex {
        self.pipeline.index()
    }

    /// Build from environment configuration, opening storage, search and the
    /// MAL client.
    pub async fn from_env() -> Result<Self, StateError> {
        let config = Config::from_env();
        let store = Store::open(kuukan_store::StoreConfig::from_env()).await?;
        let index = SearchIndex::open(&config.search_path)?;
        let mal = MalClient::new(MalConfig::from_env())?;
        Ok(AppState::new(config, store, IndexPipeline::new(index), mal))
    }
}
