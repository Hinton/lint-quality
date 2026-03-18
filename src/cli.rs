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
        #[arg(long, value_parser = ["human", "json"])]
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
}
