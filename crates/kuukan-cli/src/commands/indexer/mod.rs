//! `kuukan indexer` - port of `app/Console/Commands/Indexer/*`.
//!
//! Each subcommand crawls MyAnimeList and writes entities/cache documents into
//! the local store, mirroring the jikan indexers. Progress (the resume cursor)
//! lives in the store's `indexer_state` table; failed ids are JSON files under
//! `<data_dir>/indexer/` (jikan's `storage/app/indexer/`).

use clap::{Args, Subcommand};

pub mod anime;
pub mod anime_sweep;
pub mod common;
pub mod current_season;
pub mod genres;
pub mod incremental;
pub mod manga;
pub mod manga_sweep;
pub mod producers;
pub mod schedule;

#[derive(Args, Debug)]
pub struct IndexerArgs {
    #[command(subcommand)]
    pub command: IndexerCommand,
}

#[derive(Subcommand, Debug)]
pub enum IndexerCommand {
    /// Index all anime (uses purarue/mal-id-cache for the id list).
    Anime(IndexerOptions),
    /// Index all manga.
    Manga(IndexerOptions),
    /// Index common endpoints: producers, magazines.
    Common(IndexerOptions),
    /// Index genres (full documents in the fingerprint cache).
    Genres(IndexerOptions),
    /// Index producers (full producer documents).
    Producers(IndexerOptions),
    /// Index the weekly schedule.
    Schedule(IndexerOptions),
    /// Index the current anime season.
    CurrentSeason(IndexerOptions),
    /// Delete anime that disappeared from the MAL id cache.
    AnimeSweep(IndexerOptions),
    /// Delete manga that disappeared from the MAL id cache.
    MangaSweep(IndexerOptions),
    /// Compare the MAL id cache snapshot and index changed entries.
    Incremental(IncrementalOptions),
}

#[derive(Args, Debug, Clone)]
pub struct IndexerOptions {
    /// Seconds to wait between requests.
    #[arg(long, default_value_t = 3)]
    pub delay: u64,
    /// Start from a specific index in the id list.
    #[arg(long, default_value_t = 0)]
    pub index: usize,
    /// Start from the end of the id list.
    #[arg(long)]
    pub reverse: bool,
    /// Resume from the last saved position.
    #[arg(long)]
    pub resume: bool,
    /// Only retry entries that failed last run.
    #[arg(long)]
    pub failed: bool,
}

impl Default for IndexerOptions {
    /// The CLI defaults (`--delay=3`), also used by `kuukan schedule` for the
    /// jobs it triggers; a derived `Default` would lose the 3-second delay.
    fn default() -> Self {
        IndexerOptions {
            delay: 3,
            index: 0,
            reverse: false,
            resume: false,
            failed: false,
        }
    }
}

/// Options of `kuukan indexer incremental`.
#[derive(Args, Debug, Clone)]
pub struct IncrementalOptions {
    /// Media type(s) to check: `anime` and/or `manga`.
    #[arg(value_enum, required = true, value_name = "MEDIA")]
    pub media: Vec<common::Media>,
    /// Seconds to wait between requests.
    #[arg(long, default_value_t = 3)]
    pub delay: u64,
    /// Resume from the last saved position.
    #[arg(long)]
    pub resume: bool,
    /// Only retry entries that failed last time.
    #[arg(long)]
    pub failed: bool,
}

pub async fn run(args: IndexerArgs) -> anyhow::Result<()> {
    match args.command {
        IndexerCommand::Anime(o) => self::anime::run(o).await,
        IndexerCommand::Manga(o) => self::manga::run(o).await,
        IndexerCommand::Common(o) => self::common::run(o).await,
        IndexerCommand::Genres(o) => self::genres::run(o).await,
        IndexerCommand::Producers(o) => self::producers::run(o).await,
        IndexerCommand::Schedule(o) => self::schedule::run(o).await,
        IndexerCommand::CurrentSeason(o) => self::current_season::run(o).await,
        IndexerCommand::AnimeSweep(o) => self::anime_sweep::run(o).await,
        IndexerCommand::MangaSweep(o) => self::manga_sweep::run(o).await,
        IndexerCommand::Incremental(o) => self::incremental::run(o).await,
    }
}
