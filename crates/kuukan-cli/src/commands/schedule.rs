//! `kuukan schedule` - run the daily indexer schedule in the foreground.
//!
//! Mirrors `app/Console/Kernel.php::schedule`:
//! anime-schedule (daily), anime-current-season (daily), common (daily),
//! genres (daily), producers (daily), anime-sweep (daily), manga-sweep (daily).
//!
//! Without `--once` the schedule runs forever: one cycle immediately, then a
//! cycle every 24 hours (the `->daily()` cadence of the Laravel scheduler).
//! A failing job is logged and the cycle continues; `--once` returns an error
//! when any job failed so scripts/CI can react.

use std::time::Duration;

use clap::Args;
use kuukan_api::state::AppState;

use super::indexer::{self, IndexerOptions};

/// `->daily()` in the jikan Kernel: 24 hours between cycles.
const DAILY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Args, Debug)]
pub struct ScheduleArgs {
    /// Run every job once immediately, then exit (for testing).
    #[arg(long)]
    pub once: bool,
}

pub async fn run(args: ScheduleArgs) -> anyhow::Result<()> {
    let state = AppState::from_env().await?;

    if args.once {
        return run_cycle(&state).await;
    }

    if let Err(error) = run_cycle(&state).await {
        tracing::error!(error = %error, "scheduled indexer cycle failed");
    }

    // `tokio::time::interval` fires its first tick immediately; consume it so
    // the next cycle really happens after a full day.
    let mut interval = tokio::time::interval(DAILY);
    interval.tick().await;
    loop {
        interval.tick().await;
        if let Err(error) = run_cycle(&state).await {
            tracing::error!(error = %error, "scheduled indexer cycle failed");
        }
    }
}

/// Run every scheduled indexer job once, in `Kernel.php` order.
///
/// Jobs are independent: a failure is logged and the next job still runs. The
/// first error is returned so `--once` exits non-zero.
async fn run_cycle(state: &AppState) -> anyhow::Result<()> {
    let options = IndexerOptions::default();
    let mut first_error: Option<anyhow::Error> = None;

    macro_rules! job {
        ($name:literal, $job:expr) => {
            if let Err(error) = $job.await {
                tracing::error!(job = $name, error = %error, "scheduled job failed");
                if first_error.is_none() {
                    first_error = Some(error.context($name));
                }
            }
        };
    }

    job!(
        "indexer:anime-schedule",
        indexer::schedule::run_with_state(state, options.clone())
    );
    job!(
        "indexer:anime-current-season",
        indexer::current_season::run_with_state(state, options.clone())
    );
    job!(
        "indexer:common",
        indexer::common::run_with_state(state, options.clone())
    );
    job!(
        "indexer:genres",
        indexer::genres::run_with_state(state, options.clone())
    );
    job!(
        "indexer:producers",
        indexer::producers::run_with_state(state, options.clone())
    );
    job!(
        "indexer:anime-sweep",
        indexer::anime_sweep::run_with_state(state, options.clone())
    );
    job!(
        "indexer:manga-sweep",
        indexer::manga_sweep::run_with_state(state, options.clone())
    );

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
