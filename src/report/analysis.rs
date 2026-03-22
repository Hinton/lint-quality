//! Aggregation of per-file violations into summary counts by pattern, category,
//! rule, directory (hierarchical), and CODEOWNERS owner.

use std::collections::HashMap;
use std::path::Path;

use crate::scan::FileReport;

use super::ReportSummary;

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
        let n = file.violations.len();
        if n == 0 {
            continue;
        }
        total_violations += n;

        let owner_key = file.owner.as_deref().unwrap_or("@unowned");
        *by_owner.entry(owner_key.to_string()).or_default() += n;

        // Hierarchical directory counting: once per file, scaled by violation count.
        if let Some(parent) = Path::new(&file.path).parent() {
            let mut dir = parent.to_path_buf();
            loop {
                let dir_str = dir.to_string_lossy().into_owned();
                if dir_str.is_empty() {
                    break;
                }
                *by_directory.entry(dir_str).or_default() += n;
                if !dir.pop() {
                    break;
                }
            }
        }

        for v in &file.violations {
            *by_pattern.entry(v.pattern.clone()).or_default() += 1;
            *by_category.entry(v.category.clone()).or_default() += 1;
            for rule in &v.rules {
                *by_rule.entry(rule.clone()).or_default() += 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{FileReport, Violation};

    fn v(pattern: &str, category: &str, rules: &[&str]) -> Violation {
        Violation {
            line: 1,
            pattern: pattern.to_string(),
            category: category.to_string(),
            rules: rules.iter().map(|s| s.to_string()).collect(),
            raw_text: String::new(),
        }
    }

    fn file(path: &str, owner: Option<&str>, violations: Vec<Violation>) -> FileReport {
        FileReport {
            path: path.to_string(),
            owner: owner.map(str::to_string),
            violations,
        }
    }

    #[test]
    fn empty_input() {
        let s = build_summary(&[]);
        assert_eq!(s.total_violations, 0);
        assert_eq!(s.total_files_with_violations, 0);
        assert!(s.by_pattern.is_empty());
        assert!(s.by_owner.is_empty());
    }

    #[test]
    fn counts_by_pattern_and_category() {
        let files = vec![file(
            "src/a.ts",
            None,
            vec![
                v("ts-ignore", "typescript", &[]),
                v("ts-ignore", "typescript", &[]),
                v("eslint-disable", "eslint", &[]),
            ],
        )];
        let s = build_summary(&files);
        assert_eq!(s.total_violations, 3);
        assert_eq!(s.by_pattern["ts-ignore"], 2);
        assert_eq!(s.by_pattern["eslint-disable"], 1);
        assert_eq!(s.by_category["typescript"], 2);
        assert_eq!(s.by_category["eslint"], 1);
    }

    #[test]
    fn counts_rules() {
        let files = vec![file(
            "src/a.ts",
            None,
            vec![
                v("eslint-disable", "eslint", &["no-console", "no-unused-vars"]),
                v("eslint-disable", "eslint", &["no-console"]),
            ],
        )];
        let s = build_summary(&files);
        assert_eq!(s.by_rule["no-console"], 2);
        assert_eq!(s.by_rule["no-unused-vars"], 1);
    }

    #[test]
    fn owner_counted_once_per_file() {
        let files = vec![
            file("src/a.ts", Some("@team-a"), vec![v("ts-ignore", "ts", &[]); 3]),
            file("src/b.ts", Some("@team-a"), vec![v("ts-ignore", "ts", &[]); 2]),
        ];
        let s = build_summary(&files);
        assert_eq!(s.by_owner["@team-a"], 5);
    }

    #[test]
    fn unowned_files_use_fallback_key() {
        let files = vec![file("src/a.ts", None, vec![v("ts-ignore", "ts", &[])])];
        let s = build_summary(&files);
        assert_eq!(s.by_owner["@unowned"], 1);
    }

    #[test]
    fn hierarchical_directory_counting() {
        let files = vec![file(
            "src/foo/bar.ts",
            None,
            vec![v("ts-ignore", "ts", &[]); 3],
        )];
        let s = build_summary(&files);
        assert_eq!(s.by_directory["src/foo"], 3);
        assert_eq!(s.by_directory["src"], 3);
    }

    #[test]
    fn directory_counts_accumulate_across_files() {
        let files = vec![
            file("src/a.ts", None, vec![v("ts-ignore", "ts", &[]); 2]),
            file("src/b.ts", None, vec![v("ts-ignore", "ts", &[]); 1]),
        ];
        let s = build_summary(&files);
        assert_eq!(s.by_directory["src"], 3);
    }

    #[test]
    fn file_with_no_violations_skipped() {
        let files = vec![
            file("src/a.ts", Some("@team"), vec![]),
            file("src/b.ts", Some("@team"), vec![v("ts-ignore", "ts", &[])]),
        ];
        let s = build_summary(&files);
        assert_eq!(s.total_violations, 1);
        assert_eq!(s.by_owner["@team"], 1);
        assert!(!s.by_directory.contains_key("src/a.ts"));
    }
}
