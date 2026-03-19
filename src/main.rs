//! Entry point for lint-quality. Orchestrates the scan pipeline: config loading,
//! pattern compilation, file scanning, CODEOWNERS assignment, and report output.

mod cli;
mod config;
mod owners;
mod report;
mod scan;
mod trend;

use anyhow::{Result, bail};
use clap::Parser;
use std::io::Read;
use std::time::Instant;

use cli::{Cli, Commands};
use config::{load_config, resolve_config};
use report::Report;
use scan::compile_patterns;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            paths,
            format,
            config: config_path,
            codeowners,
            extensions,
        } => {
            let (config_file, resolved_config_path) = load_config(&config_path, &paths)?;

            let resolved = resolve_config(
                config_file,
                format.as_deref(),
                extensions.as_deref(),
                codeowners.as_deref(),
                paths,
            );

            if resolved.patterns.is_empty() {
                bail!(
                    "No patterns configured. Create a lint-quality.toml with [[patterns]] entries.\n\
                     See README.md for an example configuration."
                );
            }

            let compiled = compile_patterns(&resolved.patterns)?;

            let start_time = Instant::now();
            let mut scan_result =
                scan::scan_paths(&resolved.scan_paths, &resolved.extensions, &compiled)?;
            let duration = start_time.elapsed();

            if let Some(ref co_path) = resolved.codeowners
                && let Some(co) = owners::Owners::from_file(co_path)
            {
                owners::assign_owners(&mut scan_result.files, &co);
            }

            let report = report::build(scan_result, &resolved, resolved_config_path, duration);
            report::print(&report, &resolved.format)?;
        }

        Commands::Trend {
            paths,
            port,
            no_open,
        } => {
            trend::run(paths, port, no_open)?;
        }

        Commands::Read { path, format } => {
            let json_str = if path == "-" {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                std::fs::read_to_string(&path)?
            };

            let report: Report = serde_json::from_str(&json_str)?;
            report::print(&report, &format)?;
        }
    }

    Ok(())
}
