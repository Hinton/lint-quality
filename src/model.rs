//! Core data types shared across the scan pipeline: violations, file reports,
//! scan results, metadata, summaries, and the top-level report.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single lint suppression found in source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// 1-based line number where the suppression appears.
    pub line: usize,
    /// Name of the matched pattern (e.g. "eslint-disable-next-line", "ts-ignore").
    pub pattern: String,
    /// Category grouping the pattern (e.g. "eslint", "typescript").
    pub category: String,
    /// Specific lint rules being suppressed. Empty for patterns that don't specify rules
    /// (like `@ts-ignore`). Contains `["*"]` when all rules are suppressed.
    pub rules: Vec<String>,
    /// The raw source line text (trimmed).
    pub raw_text: String,
}

/// All violations found in a single file, along with its CODEOWNERS owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReport {
    pub path: String,
    /// CODEOWNERS owner for this file, if resolved. Omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub violations: Vec<Violation>,
}

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

/// Result of scanning one or more directory trees.
pub struct ScanResult {
    /// Files that contained at least one violation.
    pub files: Vec<FileReport>,
    /// Total files read (including those with no violations).
    pub files_scanned: usize,
}

/// Complete scan report: metadata, per-file violations, and summary aggregations.
/// Self-contained so multiple reports can be loaded for trend comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub files: Vec<FileReport>,
    pub summary: ReportSummary,
}
