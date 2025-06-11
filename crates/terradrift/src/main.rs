use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use anyhow::Context;
use crate::config::Config;
use tokio::runtime::Runtime;
use crate::orchestrator::{run_profile, WorkspaceResult};
use crate::sink::post_slack;

/// Terradrift – Terraform drift detector
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Activate verbose output (-v, -vv, etc.)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Detect drift across workspaces
    Diff {
        /// Profile (e.g., prod, staging)
        #[arg(short, long)]
        profile: String,

        /// Limit concurrency (defaults to logical CPU cores)
        #[arg(short = 'j', long)]
        jobs: Option<usize>,
    },
    /// Print build information
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

mod config;
mod provider;
mod terraform;
mod orchestrator;
mod sink;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let rt = Runtime::new()?;
    rt.block_on(async {
        match cli.command {
            Commands::Diff { profile, jobs } => {
                let config = Config::load(cli.config.clone())?;
                let prof = config.profile(&profile)?;

                let results = run_profile(&profile, prof, jobs).await?;

                // pretty print
                for r in &results {
                    println!("{:15} | drift: {:5} | changed: {:3} | {:4} ms", r.workspace, r.drift, r.changed_resources, r.duration_ms);
                }

                // emit summary json
                let summary = serde_json::json!({
                    "profile": profile,
                    "results": results,
                });
                println!("{}", serde_json::to_string_pretty(&summary)?);

                // Slack sink (optional)
                if let Ok(webhook) = std::env::var("SLACK_WEBHOOK_URL") {
                    let drift_count = results.iter().filter(|r| r.drift).count();
                    if drift_count > 0 {
                        let text = format!("🚨 Terradrift detected drift in {drift_count} workspace(s) for profile *{profile}*.");
                        let _ = post_slack(&webhook, &text).await;
                    }
                }

                // Exit code: 0 = no drift, 2 = drift detected
                if results.iter().any(|r| r.drift) {
                    std::process::exit(2);
                }
            }
            Commands::Version { json } => {
                if json {
                    let info = serde_json::json!({
                        "version": env!("CARGO_PKG_VERSION"),
                        "commit": option_env!("GIT_SHA").unwrap_or("unknown"),
                        "build_date": option_env!("BUILD_DATE").unwrap_or("unknown"),
                    });
                    println!("{}", serde_json::to_string_pretty(&info)?);
                } else {
                    println!(
                        "terradrift {} (commit: {}, built: {})",
                        env!("CARGO_PKG_VERSION"),
                        option_env!("GIT_SHA").unwrap_or("unknown"),
                        option_env!("BUILD_DATE").unwrap_or("unknown"),
                    );
                }
            }
        }
        Ok(())
    })
} 