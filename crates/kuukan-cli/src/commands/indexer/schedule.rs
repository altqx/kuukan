//! `indexer:anime-schedule` — port of
//! `App\Console\Commands\Indexer\AnimeScheduleIndexer`.
//!
//! Scrapes the weekly schedule, caches the full document under the
//! `/v1/schedules` fingerprint (default TTL) and then re-scrapes every airing
//! anime so the entries behind the schedule stay fresh.

use anyhow::Result;
use kuukan_api::config::CacheCategory;
use kuukan_api::state::AppState;
use serde_json::Value;

use super::common;
use super::IndexerOptions;

/// URI of the weekly schedule endpoint (route fingerprint base).
pub const SCHEDULE_URI: &str = "/v1/schedules";

pub async fn run(options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, options: IndexerOptions) -> Result<()> {
    let schedule = kuukan_mal::api::schedule::get_schedule(&state.mal).await?;
    let fingerprint = store_schedule_document(state, &schedule).await?;

    let ids = common::schedule_mal_ids(&schedule);
    tracing::info!(
        fingerprint = %fingerprint,
        entries = ids.len(),
        "schedule cached; refreshing airing anime"
    );

    if ids.is_empty() {
        return Ok(());
    }
    let failed_path = common::indexer_path("anime-schedule.failed.json");
    let report =
        common::refresh_anime(state, ids, &options, "anime-schedule", &failed_path).await?;
    tracing::info!(
        indexed = report.indexed,
        not_found = report.not_found,
        failed = report.failed.len(),
        "schedule refresh complete"
    );
    Ok(())
}

/// Cache the schedule document under the `/v1/schedules` fingerprint.
pub async fn store_schedule_document(state: &AppState, schedule: &Value) -> Result<String> {
    let ttl = state.config.cache_ttl(CacheCategory::Default) as i64;
    common::put_cached_document(state, "schedules", SCHEDULE_URI, ttl, schedule.clone()).await
}

#[cfg(test)]
mod tests {
    use super::super::common::test_support;
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn schedule_is_cached_under_the_route_fingerprint() {
        let (state, _dir) = test_support::state("schedule").await;
        let schedule = json!({
            "monday": [{"mal_id": 1, "title": "Airing"}],
            "tuesday": [{"mal_id": 2, "title": "Airing too"}]
        });

        let fingerprint = store_schedule_document(&state, &schedule).await.unwrap();

        // `/v1/schedules`, request type `schedules`.
        assert_eq!(
            fingerprint,
            "request:schedules:855d7d3e0d42f98b5139b26432a2ce6572040c6b"
        );
        let cached = state.store.get_cache(&fingerprint).await.unwrap().unwrap();
        assert_eq!(cached.payload["monday"][0]["mal_id"], 1);
        let ttl = state.config.cache_ttl(CacheCategory::Default) as i64;
        assert_eq!(cached.expires_at, Some(cached.modified_at + ttl));
    }

    #[test]
    fn schedule_ids_come_from_every_day() {
        let schedule = json!({
            "monday": [{"mal_id": 1}],
            "tuesday": [],
            "wednesday": [{"mal_id": 2}, {"mal_id": 3}]
        });
        assert_eq!(common::schedule_mal_ids(&schedule), vec![1, 2, 3]);
    }
}
