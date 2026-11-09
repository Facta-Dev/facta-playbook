//! `<app>-cli` — thin CLI over the SDK.
//!
//! Uses clap for arg parsing (replaces the prior raw `std::env::var`). The
//! `--base-url` flag falls back to the `APP_BASE_URL` env var, then a default.
//! ponytail: add subcommands (`widgets get`, etc.) as the SDK grows typed
//! endpoints — the clap derive here is the extension point.
use anyhow::Result;
use clap::Parser;

use <app>_sdk::Client;

/// CLI over the `<app>` HTTP API.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Base URL of the `<app>` API.
    #[arg(long, env = "APP_BASE_URL", default_value = "http://localhost:8080")]
    base_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = Client::new(cli.base_url);
    let ok = client.health().await?;
    println!("health: {ok}");
    if ok {
        Ok(())
    } else {
        // anyhow bail keeps the exit code non-zero so scripts/ci can detect.
        anyhow::bail!("health check failed: API reported unhealthy")
    }
}
