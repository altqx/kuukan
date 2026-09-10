//! `kuukan healthcheck` - verify a running instance responds.

use clap::Args;

#[derive(Args, Debug)]
pub struct HealthcheckArgs {
    #[arg(long, default_value = "http://127.0.0.1:8080/")]
    pub url: String,
}

pub async fn run(args: HealthcheckArgs) -> anyhow::Result<()> {
    let response = reqwest::get(&args.url).await?;
    if response.status().is_success() {
        println!("ok {}", response.status());
        Ok(())
    } else {
        anyhow::bail!("healthcheck failed: {}", response.status())
    }
}
