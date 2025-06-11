use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Terradrift – Terraform drift detector
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Activate verbose output (-v, -vv, etc.)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE", global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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