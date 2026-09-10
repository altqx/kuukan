//! `kuukan cache:remove` - remove cache documents by fingerprint.
//!
//! Ported from `app/Console/Commands/CacheRemove.php` (argument `key`), with an
//! added `--prefix` variant for bulk removal.

use clap::Args;
use kuukan_store::{Store, StoreConfig};

#[derive(Args, Debug)]
pub struct CacheRemoveArgs {
    /// Exact request fingerprint (`request:anime:<sha1>`).
    pub key: Option<String>,
    /// Remove every fingerprint starting with this prefix.
    #[arg(long)]
    pub prefix: Option<String>,
}

pub async fn run(args: CacheRemoveArgs) -> anyhow::Result<()> {
    if args.key.is_none() && args.prefix.is_none() {
        anyhow::bail!("provide a cache key or --prefix");
    }
    let store = Store::open(StoreConfig::from_env()).await?;

    if let Some(key) = &args.key {
        if store.delete_cache(key).await? {
            println!("Cache removed");
        } else {
            println!("Cache does not exist");
        }
    }
    if let Some(prefix) = &args.prefix {
        let removed = store.delete_cache_by_prefix(prefix).await?;
        println!("Cache removed ({removed} entries)");
    }
    Ok(())
}
