//! `api/watch.rs`: the frozen watch entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::watch::{WatchEpisodesParser, WatchPromotionalVideosParser};
use crate::request::watch::{
    PopularEpisodesRequest, PopularPromotionalVideosRequest, RecentEpisodesRequest,
    RecentPromotionalVideosRequest,
};
use crate::source::MalSource;

/// `MalClient::getRecentEpisodes(RecentEpisodesRequest $request)`.
pub async fn get_recent_episodes(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<WatchEpisodesParser>(client, RecentEpisodesRequest::new()).await
}

/// `MalClient::getPopularEpisodes(PopularEpisodesRequest $request)`.
pub async fn get_popular_episodes(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<WatchEpisodesParser>(client, PopularEpisodesRequest::new()).await
}

/// `MalClient::getRecentPromotionalVideos(RecentPromotionalVideosRequest $request)`.
pub async fn get_recent_promotional_videos(
    client: &dyn MalSource,
    page: u64,
) -> Result<Value, MalError> {
    super::fetch_and_parse::<WatchPromotionalVideosParser>(
        client,
        RecentPromotionalVideosRequest::new(page),
    )
    .await
}

/// `MalClient::getPopularPromotionalVideos(PopularPromotionalVideosRequest $request)`.
pub async fn get_popular_promotional_videos(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_and_parse::<WatchPromotionalVideosParser>(
        client,
        PopularPromotionalVideosRequest::new(),
    )
    .await
}
