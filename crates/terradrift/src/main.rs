use anyhow::Context;
use tokio::runtime::Runtime;
use clap::Parser;

use terradrift::cli::{Cli, Commands};
use terradrift::config::Config;
use terradrift::orchestrator::{run_profile, WorkspaceResult};
use terradrift::sink::post_slack;

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
                        let plan_link = std::env::var("PLAN_URL").ok();
                        let text = if let Some(url) = plan_link {
                            format!("🚨 Terradrift detected drift in {drift_count} workspace(s) for profile *{profile}*. <{url}|View plan>")
                        } else {
                            format!("🚨 Terradrift detected drift in {drift_count} workspace(s) for profile *{profile}*.")
                        };
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