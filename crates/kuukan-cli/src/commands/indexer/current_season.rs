//! `indexer:anime-current-season` — port of
//! `App\Console\Commands\Indexer\CurrentSeasonIndexer`.
//!
//! Scrapes the current season, caches the full document under the
//! `/v1/seasons/now` fingerprint (default TTL) and re-scrapes every anime in
//! that season so the entries stay fresh.

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Utc};
use kuukan_api::config::CacheCategory;
use kuukan_api::state::AppState;
use kuukan_core::enums::constants::{FALL, SPRING, SUMMER, WINTER};
use serde_json::Value;

use super::common;
use super::IndexerOptions;

/// URI of the current-season endpoint (route fingerprint base).
pub const SEASONS_NOW_URI: &str = "/v1/seasons/now";

/// MAL season of `now` as `(year, "winter"|"spring"|"summer"|"fall")`.
///
/// Like `QueryCurrentAnimeSeasonHandler` (and MAL), the season is computed in
/// `Asia/Tokyo`: Japan has no DST, so UTC+9 is exact. MAL quarters: winter =
/// Jan–Mar, spring = Apr–Jun, summer = Jul–Sep, fall = Oct–Dec.
pub fn current_season(now: DateTime<Utc>) -> (u32, &'static str) {
    let tokyo = now + Duration::hours(9);
    let season = match tokyo.month() {
        1..=3 => WINTER,
        4..=6 => SPRING,
        7..=9 => SUMMER,
        _ => FALL,
    };
    (tokyo.year() as u32, season)
}

pub async fn run(options: IndexerOptions) -> Result<()> {
    let state = AppState::from_env().await?;
    run_with_state(&state, options).await
}

/// [`run`] against an already-built state (shared by `kuukan schedule`).
pub async fn run_with_state(state: &AppState, options: IndexerOptions) -> Result<()> {
    let (year, season) = current_season(Utc::now());
    tracing::info!(year, season, "fetching current season");

    let payload = kuukan_mal::api::seasonal::get_seasonal(&state.mal, year, season, false).await?;
    let fingerprint = store_season_document(state, &payload).await?;

    let ids = common::array_mal_ids(&payload, "anime");
    tracing::info!(
        fingerprint = %fingerprint,
        entries = ids.len(),
        "current season cached; refreshing anime"
    );

    if ids.is_empty() {
        return Ok(());
    }
    let failed_path = common::indexer_path("anime-current-season.failed.json");
    let report =
        common::refresh_anime(state, ids, &options, "anime-current-season", &failed_path).await?;
    tracing::info!(
        indexed = report.indexed,
        not_found = report.not_found,
        failed = report.failed.len(),
        "current-season refresh complete"
    );
    Ok(())
}

/// Cache the seasonal document under the `/v1/seasons/now` fingerprint.
pub async fn store_season_document(state: &AppState, payload: &Value) -> Result<String> {
    let ttl = state.config.cache_ttl(CacheCategory::Default) as i64;
    common::put_cached_document(state, "seasons", SEASONS_NOW_URI, ttl, payload.clone()).await
}

#[cfg(test)]
mod tests {
    use super::super::common::test_support;
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn at(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
    }

    #[test]
    fn current_season_matches_mal_quarters_in_tokyo() {
        assert_eq!(current_season(at(2026, 1, 1, 0)), (2026, "winter"));
        assert_eq!(current_season(at(2026, 3, 31, 14)), (2026, "winter"));
        // 2026-03-31T16:00Z = 2026-04-01T01:00 JST: already spring.
        assert_eq!(current_season(at(2026, 3, 31, 16)), (2026, "spring"));
        assert_eq!(current_season(at(2026, 6, 30, 14)), (2026, "spring"));
        assert_eq!(current_season(at(2026, 6, 30, 16)), (2026, "summer"));
        assert_eq!(current_season(at(2026, 9, 30, 14)), (2026, "summer"));
        assert_eq!(current_season(at(2026, 9, 30, 16)), (2026, "fall"));
        assert_eq!(current_season(at(2026, 12, 31, 14)), (2026, "fall"));
        // 2025-12-31T16:00Z = 2026-01-01T01:00 JST: the year rolls over too.
        assert_eq!(current_season(at(2025, 12, 31, 16)), (2026, "winter"));
    }

    #[tokio::test]
    async fn season_document_uses_the_seasons_now_fingerprint() {
        let (state, _dir) = test_support::state("current-season").await;
        let payload = json!({"anime": [{"mal_id": 1, "title": "Airing"}]});

        let fingerprint = store_season_document(&state, &payload).await.unwrap();

        // `/v1/seasons/now`, request type `seasons`.
        assert_eq!(
            fingerprint,
            "request:seasons:88d3411b5819cb9332f83491529f79da4d71fb41"
        );
        let cached = state.store.get_cache(&fingerprint).await.unwrap().unwrap();
        assert_eq!(cached.payload["anime"][0]["mal_id"], 1);
        let ttl = state.config.cache_ttl(CacheCategory::Default) as i64;
        assert_eq!(cached.expires_at, Some(cached.modified_at + ttl));
    }
}
