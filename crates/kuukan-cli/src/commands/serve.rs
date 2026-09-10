//! `kuukan serve` - run the HTTP API.

use clap::Args;

#[derive(Args, Debug)]
pub struct ServeArgs {
    /// Address to listen on.
    #[arg(long, default_value = "0.0.0.0:8080")]
    pub listen: String,
}

pub async fn run(args: ServeArgs) -> anyhow::Result<()> {
    kuukan_api::server::serve(&args.listen)
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))?;
    Ok(())
}
