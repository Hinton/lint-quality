//! File discovery and line-by-line violation scanning.
//!
//! The scan pipeline runs in two stages:
//!
//! 1. **Discovery** — [`scan_paths`] builds a unified directory walker (via the `ignore` crate,
//!    respecting `.gitignore`) and filters entries down to files with allowed extensions.
//! 2. **Matching** — [`scan_file_contents`] tests every line against all compiled patterns,
//!    collecting one [`Violation`] per match.

use anyhow::Result;
use ignore::{Walk, WalkBuilder};
use std::path::Path;

use super::model::{FileReport, ScanResult, Violation};
use super::patterns::CompiledPattern;

/// Walk the given paths, read files matching the allowed extensions, and return
/// all detected violations along with the total number of files scanned.
///
/// The walker respects `.gitignore` and skips hidden files. Files that cannot be
/// read as UTF-8 are skipped with a warning to stderr. Files with no violations
/// are not included in [`ScanResult::files`], but do count toward
/// [`ScanResult::files_scanned`].
pub fn scan_paths(
    paths: &[impl AsRef<Path>],
    extensions: &[String],
    patterns: &[CompiledPattern],
) -> Result<ScanResult> {
    let Some(walker) = build_walker(paths) else {
        return Ok(ScanResult { files: Vec::new(), files_scanned: 0 });
    };

    let mut files = Vec::new();
    let mut files_scanned: usize = 0;

    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let file_path = entry.path();

        if !has_allowed_extension(file_path, extensions) {
            continue;
        }

        files_scanned += 1;

        if let Some(report) = read_and_scan(file_path, patterns) {
            files.push(report);
        }
    }

    Ok(ScanResult { files, files_scanned })
}

/// Build a single walker over all `paths`, respecting `.gitignore` and skipping hidden files.
/// Returns `None` if `paths` is empty.
fn build_walker(paths: &[impl AsRef<Path>]) -> Option<Walk> {
    let (first, rest) = paths.split_first()?;
    let mut builder = WalkBuilder::new(first.as_ref());
    for path in rest {
        builder.add(path.as_ref());
    }
    Some(builder.hidden(true).git_ignore(true).git_global(true).build())
}

/// Read a file, scan it for violations, and return a [`FileReport`] if any are found.
/// Returns `None` if the file cannot be read (warning printed to stderr) or has no violations.
fn read_and_scan(path: &Path, patterns: &[CompiledPattern]) -> Option<FileReport> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: skipping {}: {}", path.display(), e);
            return None;
        }
    };
    let violations = scan_file_contents(&contents, patterns);
    if violations.is_empty() {
        return None;
    }
    Some(FileReport {
        path: path.to_string_lossy().to_string(),
        owner: None, // filled in later by the owners module
        violations,
    })
}

/// Returns `true` if the file's extension (case-sensitive) is in `extensions`.
fn has_allowed_extension(path: &Path, extensions: &[String]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| extensions.iter().any(|allowed| allowed == ext))
}

/// Scan the text contents of a single file and return all matching violations.
///
/// Each line is tested against every pattern in order. Multiple patterns may
/// match the same line, producing one [`Violation`] per match.
fn scan_file_contents(contents: &str, patterns: &[CompiledPattern]) -> Vec<Violation> {
    contents
        .lines()
        .enumerate()
        .flat_map(|(line_num, line)| {
            patterns.iter().filter_map(move |pattern| {
                pattern.regex.captures(line).map(|caps| Violation {
                    line: line_num + 1,
                    pattern: pattern.name.clone(),
                    category: pattern.category.clone(),
                    rules: pattern.extract_rules_from_match(&caps),
                    raw_text: line.trim().to_string(),
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn make_pattern(name: &str, category: &str, regex: &str, extract_rules: bool) -> CompiledPattern {
        CompiledPattern {
            name: name.to_string(),
            category: category.to_string(),
            regex: Regex::new(regex).unwrap(),
            extract_rules,
        }
    }

    // --- scan_file_contents ---

    #[test]
    fn no_patterns_returns_empty() {
        let violations = scan_file_contents("// eslint-disable-next-line\nfoo()", &[]);
        assert!(violations.is_empty());
    }

    #[test]
    fn no_matches_returns_empty() {
        let p = make_pattern("ts-ignore", "typescript", r"@ts-ignore", false);
        let violations = scan_file_contents("// normal comment\nfoo()", &[p]);
        assert!(violations.is_empty());
    }

    #[test]
    fn matching_line_produces_violation() {
        let p = make_pattern("ts-ignore", "typescript", r"@ts-ignore", false);
        let violations = scan_file_contents("// @ts-ignore\nfoo()", &[p]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].line, 1);
        assert_eq!(violations[0].pattern, "ts-ignore");
        assert_eq!(violations[0].category, "typescript");
    }

    #[test]
    fn line_numbers_are_one_based() {
        let p = make_pattern("ts-ignore", "typescript", r"@ts-ignore", false);
        let violations = scan_file_contents("foo()\nbar()\n// @ts-ignore", &[p]);
        assert_eq!(violations[0].line, 3);
    }

    #[test]
    fn raw_text_is_trimmed() {
        let p = make_pattern("ts-ignore", "typescript", r"@ts-ignore", false);
        let violations = scan_file_contents("    // @ts-ignore   ", &[p]);
        assert_eq!(violations[0].raw_text, "// @ts-ignore");
    }

    #[test]
    fn extracts_rules_from_capture_group() {
        let p = make_pattern("eslint-disable", "eslint", r"eslint-disable-next-line\s+(.*)", true);
        let violations =
            scan_file_contents("// eslint-disable-next-line no-console, no-unused-vars", &[p]);
        assert_eq!(violations[0].rules, vec!["no-console", "no-unused-vars"]);
    }

    #[test]
    fn no_rules_captured_returns_wildcard() {
        let p = make_pattern("ts-ignore", "typescript", r"@ts-ignore", true);
        let violations = scan_file_contents("// @ts-ignore", &[p]);
        assert_eq!(violations[0].rules, vec!["*"]);
    }

    #[test]
    fn multiple_patterns_can_match_same_line() {
        let p1 = make_pattern("ts-ignore", "typescript", r"@ts-ignore", false);
        let p2 = make_pattern("ts-prefix", "typescript", r"@ts-", false);
        let violations = scan_file_contents("// @ts-ignore", &[p1, p2]);
        assert_eq!(violations.len(), 2);
    }

    // --- has_allowed_extension ---

    #[test]
    fn matches_allowed_extension() {
        let exts = vec!["ts".to_string(), "js".to_string()];
        assert!(has_allowed_extension(Path::new("foo/bar.ts"), &exts));
    }

    #[test]
    fn rejects_disallowed_extension() {
        let exts = vec!["ts".to_string()];
        assert!(!has_allowed_extension(Path::new("foo/bar.rs"), &exts));
    }

    #[test]
    fn rejects_path_with_no_extension() {
        let exts = vec!["ts".to_string()];
        assert!(!has_allowed_extension(Path::new("Makefile"), &exts));
    }
}
