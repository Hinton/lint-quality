//! Core data types for the scan domain: violations, file reports, and scan results.

use serde::{Deserialize, Serialize};

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

/// Result of scanning one or more directory trees.
pub struct ScanResult {
    /// Files that contained at least one violation.
    pub files: Vec<FileReport>,
    /// Total files read (including those with no violations).
    pub files_scanned: usize,
}
