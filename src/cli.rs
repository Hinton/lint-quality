//! CLI argument parsing using clap's derive API.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Top-level CLI definition.
#[derive(Debug, Parser)]
#[command(
    name = "lint-quality",
    about = "Detect disabled lint rules in codebases"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Scan directories for lint suppressions
    Scan {
        /// Paths to scan (defaults to config scan_paths or current directory)
        #[arg()]
        paths: Vec<PathBuf>,

        /// Output format
        #[arg(long, value_parser = ["human", "json", "tui"])]
        format: Option<String>,

        /// Path to config file (overrides auto-discovery)
        #[arg(long)]
        config: Option<PathBuf>,

        /// Path to CODEOWNERS file
        #[arg(long)]
        codeowners: Option<PathBuf>,

        /// File extensions to scan (comma-separated)
        #[arg(long, value_delimiter = ',')]
        extensions: Option<Vec<String>>,
    },

    /// Launch a web dashboard to visualize trends across multiple reports
    Trend {
        /// Paths to JSON report files or directories containing them
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Port for the web server
        #[arg(long, default_value = "8081")]
        port: u16,

        /// Don't automatically open the browser
        #[arg(long)]
        no_open: bool,
    },

    /// Read a previously saved JSON report
    Read {
        /// Path to the JSON report file (use "-" for stdin)
        #[arg()]
        path: String,

        /// Output format
        #[arg(long, value_parser = ["human", "json"], default_value = "human")]
        format: String,
    },
}
