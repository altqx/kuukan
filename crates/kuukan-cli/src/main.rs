mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kuukan",
    version,
    about = "Kuukan - a Rust rewrite of the Jikan REST API v4"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP API server.
    Serve(commands::serve::ServeArgs),
    /// Remove a cache entry by fingerprint or prefix.
    #[command(name = "cache:remove", alias = "cache-remove")]
    CacheRemove(commands::cache_remove::CacheRemoveArgs),
    /// Import a JSON dump of Jikan payloads into the local store.
    Import(commands::import::ImportArgs),
    /// Crawl/refresh MyAnimeList data (jikan's indexer:* commands).
    Indexer(commands::indexer::IndexerArgs),
    /// Run the daily schedule in the foreground.
    Schedule(commands::schedule::ScheduleArgs),
    /// Check that a running kuukan instance responds.
    Healthcheck(commands::healthcheck::HealthcheckArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(args) => commands::serve::run(args).await,
        Command::CacheRemove(args) => commands::cache_remove::run(args).await,
        Command::Import(args) => commands::import::run(args).await,
        Command::Indexer(args) => commands::indexer::run(args).await,
        Command::Schedule(args) => commands::schedule::run(args).await,
        Command::Healthcheck(args) => commands::healthcheck::run(args).await,
    }
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
