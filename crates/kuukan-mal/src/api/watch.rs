//! `api/watch.rs`: the frozen watch entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::watch::{WatchEpisodesParser, WatchPromotionalVideosParser};
use crate::request::watch::{
    PopularEpisodesRequest, PopularPromotionalVideosRequest, RecentEpisodesRequest,
    RecentPromotionalVideosRequest,
};
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getRecentEpisodes(RecentEpisodesRequest $request)`.
pub async fn get_recent_episodes(client: &MalClient) -> Result<Value, MalError> {
    let path = RecentEpisodesRequest::new().path();
    let doc = client.get_html(&path).await?;
    WatchEpisodesParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getPopularEpisodes(PopularEpisodesRequest $request)`.
pub async fn get_popular_episodes(client: &MalClient) -> Result<Value, MalError> {
    let path = PopularEpisodesRequest::new().path();
    let doc = client.get_html(&path).await?;
    WatchEpisodesParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getRecentPromotionalVideos(RecentPromotionalVideosRequest $request)`.
pub async fn get_recent_promotional_videos(
    client: &MalClient,
    page: u64,
) -> Result<Value, MalError> {
    let path = RecentPromotionalVideosRequest::new(page).path();
    let doc = client.get_html(&path).await?;
    WatchPromotionalVideosParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getPopularPromotionalVideos(PopularPromotionalVideosRequest $request)`.
pub async fn get_popular_promotional_videos(client: &MalClient) -> Result<Value, MalError> {
    let path = PopularPromotionalVideosRequest::new().path();
    let doc = client.get_html(&path).await?;
    WatchPromotionalVideosParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
