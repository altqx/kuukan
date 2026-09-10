//! Source heartbeat and failover state (`source_health` table).
//!
//! Ported from `App\Providers\SourceHeartbeatProvider` and
//! `App\Listeners\SourceHeartbeatListener` of jikan-rest. Every MAL request
//! records a good/bad result; when the successful-request score drops to 0.25
//! or below, failover is enabled and the API serves the stale database copy.
//! Once `SOURCE_BAD_HEALTH_RECHECK` seconds have passed, a healthy score
//! disables failover again.
//!
//! Ported quirks (kept intentionally):
//!
//! * Fail records are `[timestamp, status, health]` with `health = 0` good and
//!   `1` bad (`SourceHeartbeatEvent` constants). Here `status` is always `0`
//!   because [`Store::record_source_result`] only receives a good/bad flag.
//! * The score denominator is `count - 1` (not `count`), so a single good
//!   request scores `1.0` and two good requests score `2.0`.
//! * `SOURCE_BAD_HEALTH_RANGE` is parsed by the PHP but its pruning loop is a
//!   no-op (`unset($fail)` on a foreach copy), so records never age out by
//!   range; only `SOURCE_BAD_HEALTH_MAX_STORE` trims the list. Kuukan keeps the
//!   same behavior.

use serde::{Deserialize, Serialize};

use crate::config::{env_f64, env_i64};
use crate::db::{now_unix, Store, StoreError};

/// `SourceHeartbeatEvent::GOOD_HEALTH`.
pub const GOOD_HEALTH: i64 = 0;
/// `SourceHeartbeatEvent::BAD_HEALTH`.
pub const BAD_HEALTH: i64 = 1;

/// Score at or below which failover is enabled
/// (`SourceHeartbeatListener::handle` hardcodes `<= 0.25`).
pub const FAILOVER_SCORE: f64 = 0.25;

/// Fallback for `SOURCE_GOOD_HEALTH_SCORE` when the variable is unset.
pub const DEFAULT_GOOD_HEALTH_SCORE: f64 = 0.9;

/// Fallback for `SOURCE_BAD_HEALTH_MAX_STORE`.
pub const DEFAULT_BAD_HEALTH_MAX_STORE: i64 = 50;

/// Fallback for `SOURCE_BAD_HEALTH_RECHECK`.
pub const DEFAULT_BAD_HEALTH_RECHECK: i64 = 10;

/// One heartbeat record, stored in the `fails` JSON array as the tuple
/// `[timestamp, status, health]` exactly like `failovers.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "[i64; 3]", into = "[i64; 3]")]
pub struct HealthRecord {
    /// Unix seconds of the request.
    pub at: i64,
    /// HTTP status of the failed request (PHP); always `0` in Kuukan.
    pub status: i64,
    /// [`GOOD_HEALTH`] or [`BAD_HEALTH`].
    pub health: i64,
}

impl HealthRecord {
    /// A successful request (`health = GOOD_HEALTH`).
    pub fn good(at: i64, status: i64) -> Self {
        Self {
            at,
            status,
            health: GOOD_HEALTH,
        }
    }

    /// A failed request (`health = BAD_HEALTH`).
    pub fn bad(at: i64, status: i64) -> Self {
        Self {
            at,
            status,
            health: BAD_HEALTH,
        }
    }

    /// Whether this record is a successful request.
    pub fn is_good(&self) -> bool {
        self.health == GOOD_HEALTH
    }
}

impl From<[i64; 3]> for HealthRecord {
    fn from(value: [i64; 3]) -> Self {
        Self {
            at: value[0],
            status: value[1],
            health: value[2],
        }
    }
}

impl From<HealthRecord> for [i64; 3] {
    fn from(value: HealthRecord) -> Self {
        [value.at, value.status, value.health]
    }
}

/// Aggregated source health, mirroring the `failovers.json` +
/// `source_failover.lock` + `source_failover_last_downtime` trio.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Health {
    /// Heartbeat records, oldest first.
    pub fails: Vec<HealthRecord>,
    /// Whether failover (serve stale database copy) is enabled.
    pub failover: bool,
    /// Unix seconds when failover was last enabled (`0` if never).
    pub last_downtime: i64,
    /// Unix seconds of the last state change.
    pub updated_at: i64,
}

impl Health {
    /// Successful-request score, exactly like `SourceHeartbeatProvider::getHeartbeatScore`:
    /// `good / max(count - 1, 1)`.
    pub fn score(&self) -> f64 {
        let good = self.fails.iter().filter(|record| record.is_good()).count() as f64;
        let denominator = (self.fails.len() as i64 - 1).max(1) as f64;
        good / denominator
    }

    /// Heartbeat status string (`HEALTHY`, `LEARNING`, `UNHEALTHY`) using
    /// `SOURCE_GOOD_HEALTH_SCORE`.
    pub fn status(&self) -> &'static str {
        self.status_with(env_f64(
            "SOURCE_GOOD_HEALTH_SCORE",
            DEFAULT_GOOD_HEALTH_SCORE,
        ))
    }

    /// Heartbeat status with an explicit good-health threshold, mirroring
    /// `SourceHeartbeatProvider::getHeartbeatStatus`.
    pub fn status_with(&self, good_health_score: f64) -> &'static str {
        let score = self.score();
        if score > 0.5 && score < good_health_score {
            "LEARNING"
        } else if score <= 0.5 {
            "UNHEALTHY"
        } else {
            "HEALTHY"
        }
    }
}

/// Tunables for [`Store::record_source_result`], read from the environment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealthThresholds {
    /// `SOURCE_BAD_HEALTH_MAX_STORE`: keep at most this many records
    /// (`0` keeps all, like the PHP `array_slice` edge case).
    pub max_store: usize,
    /// `SOURCE_BAD_HEALTH_RECHECK`: seconds before failover may be disabled.
    pub recheck_secs: i64,
    /// `SOURCE_GOOD_HEALTH_SCORE`: score needed to disable failover and to
    /// report `HEALTHY`.
    pub good_score: f64,
    /// `SourceHeartbeatListener` hardcoded enable threshold (0.25).
    pub failover_score: f64,
}

impl HealthThresholds {
    /// Read thresholds from the environment, falling back to the `.env.dist`
    /// defaults.
    pub fn from_env() -> Self {
        Self {
            max_store: env_i64("SOURCE_BAD_HEALTH_MAX_STORE", DEFAULT_BAD_HEALTH_MAX_STORE).max(0)
                as usize,
            recheck_secs: env_i64("SOURCE_BAD_HEALTH_RECHECK", DEFAULT_BAD_HEALTH_RECHECK).max(0),
            good_score: env_f64("SOURCE_GOOD_HEALTH_SCORE", DEFAULT_GOOD_HEALTH_SCORE),
            failover_score: FAILOVER_SCORE,
        }
    }
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self::from_env()
    }
}

#[derive(sqlx::FromRow)]
struct HealthRow {
    fails: String,
    failover: i64,
    last_downtime: i64,
    updated_at: i64,
}

impl Store {
    /// Read the current health state. Missing state reads as [`Health::default`].
    pub async fn get_health(&self) -> Result<Health, StoreError> {
        let row = sqlx::query_as::<_, HealthRow>(
            "SELECT fails, failover, last_downtime, updated_at \
             FROM source_health WHERE id = 1",
        )
        .fetch_optional(self.pool())
        .await?;

        match row {
            Some(row) => Ok(Health {
                fails: serde_json::from_str(&row.fails)?,
                failover: row.failover != 0,
                last_downtime: row.last_downtime,
                updated_at: row.updated_at,
            }),
            None => Ok(Health::default()),
        }
    }

    /// Record one MAL request result and update failover state.
    ///
    /// Replicates `SourceHeartbeatListener`: if failover is enabled and the
    /// recheck window (`SOURCE_BAD_HEALTH_RECHECK`) has elapsed, a recovered
    /// score (`>= SOURCE_GOOD_HEALTH_SCORE`) disables failover and clears the
    /// records; the new result is then appended (capped at
    /// `SOURCE_BAD_HEALTH_MAX_STORE`); finally, a score `<= 0.25` enables
    /// failover and stamps `last_downtime = now`.
    pub async fn record_source_result(
        &self,
        good: bool,
        now_i64: i64,
    ) -> Result<Health, StoreError> {
        let thresholds = HealthThresholds::from_env();
        self.record_source_result_at(good, now_i64, &thresholds)
            .await
    }

    /// [`Store::record_source_result`] with explicit thresholds (for tests).
    pub(crate) async fn record_source_result_at(
        &self,
        good: bool,
        now: i64,
        thresholds: &HealthThresholds,
    ) -> Result<Health, StoreError> {
        let mut health = self.get_health().await?;

        // SourceHeartbeatListener::__construct.
        if health.failover
            && now > health.last_downtime.saturating_add(thresholds.recheck_secs)
            && health.score() >= thresholds.good_score
        {
            health.failover = false;
            health.fails.clear();
        }

        health.fails.push(if good {
            HealthRecord::good(now, 0)
        } else {
            HealthRecord::bad(now, 0)
        });

        if thresholds.max_store > 0 && health.fails.len() > thresholds.max_store {
            let excess = health.fails.len() - thresholds.max_store;
            health.fails.drain(..excess);
        }

        // SourceHeartbeatListener::handle.
        if health.score() <= thresholds.failover_score {
            health.failover = true;
            health.last_downtime = now;
        }

        health.updated_at = now;
        self.save_health(&health).await?;
        Ok(health)
    }

    /// Force failover on or off.
    ///
    /// Enabling mirrors `enableFailover()` and stamps `last_downtime = now`;
    /// disabling mirrors `disableFailover()` and clears the heartbeat records.
    pub async fn set_failover(&self, enabled: bool) -> Result<(), StoreError> {
        let now = now_unix();
        let mut health = self.get_health().await?;
        if enabled {
            health.failover = true;
            health.last_downtime = now;
        } else {
            health.failover = false;
            health.fails.clear();
        }
        health.updated_at = now;
        self.save_health(&health).await
    }

    /// Unix seconds of the last failover activation (`0` if never).
    pub async fn last_downtime(&self) -> Result<i64, StoreError> {
        Ok(self.get_health().await?.last_downtime)
    }

    async fn save_health(&self, health: &Health) -> Result<(), StoreError> {
        let fails = serde_json::to_string(&health.fails)?;
        sqlx::query(
            "INSERT INTO source_health (id, fails, failover, last_downtime, updated_at) \
             VALUES (1, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 fails = excluded.fails, \
                 failover = excluded.failover, \
                 last_downtime = excluded.last_downtime, \
                 updated_at = excluded.updated_at",
        )
        .bind(fails)
        .bind(i64::from(health.failover))
        .bind(health.last_downtime)
        .bind(health.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health(healths: &[i64]) -> Health {
        Health {
            fails: healths
                .iter()
                .enumerate()
                .map(|(index, value)| HealthRecord {
                    at: index as i64,
                    status: 0,
                    health: *value,
                })
                .collect(),
            ..Health::default()
        }
    }

    #[test]
    fn score_matches_php_denominator_quirk() {
        assert_eq!(health(&[]).score(), 0.0);
        assert_eq!(health(&[GOOD_HEALTH]).score(), 1.0);
        // Two goods score 2.0 in the PHP implementation (`2 / max(1, 1)`).
        assert_eq!(health(&[GOOD_HEALTH, GOOD_HEALTH]).score(), 2.0);
        assert_eq!(health(&[BAD_HEALTH, GOOD_HEALTH]).score(), 1.0);
        // 1 good / max(3 - 1, 1) = 0.5.
        assert_eq!(health(&[GOOD_HEALTH, BAD_HEALTH, BAD_HEALTH]).score(), 0.5);
        // 2 good / 3.
        assert!(
            (health(&[GOOD_HEALTH, GOOD_HEALTH, BAD_HEALTH, BAD_HEALTH]).score() - 2.0 / 3.0).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn status_matches_php_thresholds() {
        assert_eq!(health(&[]).status_with(0.9), "UNHEALTHY");
        assert_eq!(health(&[GOOD_HEALTH]).status_with(0.9), "HEALTHY");
        assert_eq!(
            health(&[GOOD_HEALTH, BAD_HEALTH, BAD_HEALTH]).status_with(0.9),
            "UNHEALTHY"
        );
        assert_eq!(
            health(&[GOOD_HEALTH, GOOD_HEALTH, BAD_HEALTH, BAD_HEALTH]).status_with(0.9),
            "LEARNING"
        );
        // With an unusually high threshold, a score between 0.5 and the
        // threshold is still LEARNING (`score > 0.5 && score < threshold`).
        assert_eq!(health(&[GOOD_HEALTH]).status_with(2.0), "LEARNING");
    }

    #[test]
    fn health_records_serialize_as_php_tuples() {
        let records = vec![
            HealthRecord::good(1_000, 200),
            HealthRecord::bad(1_001, 503),
        ];
        let json = serde_json::to_value(&records).expect("serialize");
        assert_eq!(json, serde_json::json!([[1000, 200, 0], [1001, 503, 1]]));
        let parsed: Vec<HealthRecord> = serde_json::from_value(json).expect("deserialize");
        assert_eq!(parsed, records);
    }

    #[tokio::test]
    async fn record_source_result_enables_and_recovers_failover() {
        let store = Store::open_in_memory().await.expect("open");
        let thresholds = HealthThresholds {
            max_store: 50,
            recheck_secs: 10,
            good_score: 0.9,
            failover_score: FAILOVER_SCORE,
        };

        assert_eq!(store.get_health().await.expect("health"), Health::default());

        let failing = store
            .record_source_result_at(false, 1_000, &thresholds)
            .await
            .expect("record");
        assert!(failing.failover);
        assert_eq!(failing.last_downtime, 1_000);
        assert_eq!(failing.fails.len(), 1);

        // Inside the recheck window: no recovery attempt.
        let still_failing = store
            .record_source_result_at(true, 1_005, &thresholds)
            .await
            .expect("record");
        assert!(still_failing.failover);
        assert_eq!(still_failing.fails.len(), 2);

        // Outside the recheck window the recovered score disables failover and
        // clears the old records before appending this one.
        let recovered = store
            .record_source_result_at(true, 1_016, &thresholds)
            .await
            .expect("record");
        assert!(!recovered.failover);
        assert_eq!(recovered.fails.len(), 1);
        assert!(recovered.fails[0].is_good());
        assert_eq!(store.last_downtime().await.expect("downtime"), 1_000);
    }

    #[tokio::test]
    async fn records_are_capped_at_max_store() {
        let store = Store::open_in_memory().await.expect("open");
        let thresholds = HealthThresholds {
            max_store: 3,
            recheck_secs: 10,
            good_score: 0.9,
            failover_score: FAILOVER_SCORE,
        };
        for now in 1_000..1_005 {
            store
                .record_source_result_at(false, now, &thresholds)
                .await
                .expect("record");
        }
        let health = store.get_health().await.expect("health");
        assert_eq!(health.fails.len(), 3);
        assert_eq!(health.fails.first().map(|record| record.at), Some(1_002));
    }

    #[tokio::test]
    async fn set_failover_toggles_and_clears_records() {
        let store = Store::open_in_memory().await.expect("open");
        store.record_source_result(true, 500).await.expect("record");

        store.set_failover(true).await.expect("enable");
        let enabled = store.get_health().await.expect("health");
        assert!(enabled.failover);
        assert!(enabled.last_downtime > 0);
        assert!(enabled.last_downtime <= now_unix());
        assert_eq!(enabled.fails.len(), 1);

        store.set_failover(false).await.expect("disable");
        let disabled = store.get_health().await.expect("health");
        assert!(!disabled.failover);
        assert!(disabled.fails.is_empty());
    }
}
