//! Aggregation of per-file violations into summary counts by pattern, category,
//! rule, directory (hierarchical), and CODEOWNERS owner.

use std::collections::HashMap;
use std::path::Path;

use crate::model::{FileReport, ReportSummary};

/// Build a summary with violation counts broken down by multiple dimensions.
/// Directory counts are hierarchical: each violation increments all ancestor directories.
pub fn build_summary(files: &[FileReport]) -> ReportSummary {
    let mut by_pattern: HashMap<String, usize> = HashMap::new();
    let mut by_category: HashMap<String, usize> = HashMap::new();
    let mut by_rule: HashMap<String, usize> = HashMap::new();
    let mut by_directory: HashMap<String, usize> = HashMap::new();
    let mut by_owner: HashMap<String, usize> = HashMap::new();
    let mut total_violations: usize = 0;

    for file in files {
        let owner_key = file.owner.clone().unwrap_or_else(|| "@unowned".to_string());

        for v in &file.violations {
            total_violations += 1;

            *by_pattern.entry(v.pattern.clone()).or_default() += 1;
            *by_category.entry(v.category.clone()).or_default() += 1;
            *by_owner.entry(owner_key.clone()).or_default() += 1;

            for rule in &v.rules {
                *by_rule.entry(rule.clone()).or_default() += 1;
            }

            // Hierarchical directory counting
            if let Some(parent) = Path::new(&file.path).parent() {
                let mut dir = parent.to_path_buf();
                loop {
                    let dir_str = dir.to_string_lossy().to_string();
                    if dir_str.is_empty() {
                        break;
                    }
                    *by_directory.entry(dir_str).or_default() += 1;
                    if !dir.pop() {
                        break;
                    }
                }
            }
        }
    }

    ReportSummary {
        total_violations,
        total_files_with_violations: files.len(),
        by_pattern,
        by_category,
        by_rule,
        by_directory,
        by_owner,
    }
}
