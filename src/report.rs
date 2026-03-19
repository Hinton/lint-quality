//! Report domain: report types, construction, analysis, and output formatting.

pub(crate) mod analysis;
pub mod output;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::config::ResolvedConfig;
use crate::scan::{FileReport, ScanResult};

/// Metadata about a scan run, included in every report for traceability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub timestamp: DateTime<Utc>,
    pub tool_version: String,
    pub scanned_paths: Vec<String>,
    /// Path to the config file used, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    /// Total number of files with matching extensions that were read.
    pub files_scanned: usize,
    pub scan_duration_ms: u64,
}

/// Pre-computed aggregate counts for the report, broken down by multiple dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_violations: usize,
    pub total_files_with_violations: usize,
    pub by_pattern: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub by_rule: HashMap<String, usize>,
    /// Hierarchical directory counts — each violation increments all ancestor directories.
    pub by_directory: HashMap<String, usize>,
    pub by_owner: HashMap<String, usize>,
}

/// Complete scan report: metadata, per-file violations, and summary aggregations.
/// Self-contained so multiple reports can be loaded for trend comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub files: Vec<FileReport>,
    pub summary: ReportSummary,
}

/// Assemble a report from scan results, resolved config, and timing info.
pub fn build(
    scan_result: ScanResult,
    resolved: &ResolvedConfig,
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

/// Print a report in the given format ("human", "json", or "tui").
pub fn print(report: &Report, format: &str) -> Result<()> {
    match format {
        "json" => println!("{}", output::json::format_json(report)?),
        "tui" => output::tui::run_tui(report)?,
        _ => print!("{}", output::human::format_human(report)),
    }
    Ok(())
}
