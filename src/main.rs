//! Entry point for lint-quality. Orchestrates the scan pipeline: config loading,
//! pattern compilation, file scanning, CODEOWNERS assignment, and report output.

mod analysis;
mod cli;
mod config;
mod model;
mod output;
mod owners;
mod patterns;
mod scanner;

use anyhow::{Result, bail};
use chrono::Utc;
use clap::Parser;
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use cli::{Cli, Commands};
use config::{ConfigFile, discover_config, load_config_file, resolve_config};
use model::{Report, ReportMetadata, ScanResult};
use output::{human, json, tui};
use patterns::compile_patterns;

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
                scanner::scan_paths(&resolved.scan_paths, &resolved.extensions, &compiled)?;
            let duration = start_time.elapsed();

            if let Some(ref co_path) = resolved.codeowners
                && let Some(co) = owners::Owners::from_file(co_path)
            {
                owners::assign_owners(&mut scan_result.files, &co);
            }

            let report = build_report(scan_result, &resolved, resolved_config_path, duration);

            print_report(&report, &resolved.format)?;
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
            print_report(&report, &format)?;
        }
    }

    Ok(())
}

fn print_report(report: &Report, format: &str) -> Result<()> {
    match format {
        "json" => println!("{}", json::format_json(report)?),
        "tui" => tui::run_tui(report)?,
        _ => print!("{}", human::format_human(report)),
    }
    Ok(())
}

/// Load the config file, either from an explicit path or by auto-discovering
/// `lint-quality.toml` in parent directories of the scan target.
fn load_config(
    config_path: &Option<PathBuf>,
    scan_paths: &[PathBuf],
) -> Result<(Option<ConfigFile>, Option<String>)> {
    match config_path {
        Some(p) => Ok((
            Some(load_config_file(p)?),
            Some(p.to_string_lossy().to_string()),
        )),
        None => {
            let start = scan_paths
                .first()
                .map(|p| p.as_path())
                .unwrap_or(".".as_ref());
            match discover_config(start) {
                Some(p) => {
                    let display = p.to_string_lossy().to_string();
                    Ok((load_config_file(&p).ok(), Some(display)))
                }
                None => Ok((None, None)),
            }
        }
    }
}

/// Assemble the final report from scan results, resolved config, and timing info.
fn build_report(
    scan_result: ScanResult,
    resolved: &config::ResolvedConfig,
    config_path: Option<String>,
    duration: Duration,
) -> Report {
    let summary = analysis::build_summary(&scan_result.files);
    Report {
        metadata: ReportMetadata {
            timestamp: Utc::now(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            scanned_paths: resolved
                .scan_paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            config_path,
            files_scanned: scan_result.files_scanned,
            scan_duration_ms: duration.as_millis() as u64,
        },
        files: scan_result.files,
        summary,
    }
}
