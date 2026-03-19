//! File discovery and line-by-line violation scanning.
//!
//! Uses the `ignore` crate to walk directories while respecting `.gitignore`.
//! Files are filtered by extension, then each line is tested against compiled patterns.

use anyhow::Result;
use ignore::WalkBuilder;
use std::path::Path;

use super::model::{FileReport, ScanResult, Violation};
use super::patterns::CompiledPattern;

/// Walk the given paths, read files matching the allowed extensions, and return
/// all detected violations along with the total number of files scanned.
pub fn scan_paths(
    paths: &[impl AsRef<Path>],
    extensions: &[String],
    patterns: &[CompiledPattern],
) -> Result<ScanResult> {
    let mut files = Vec::new();
    let mut files_scanned: usize = 0;

    for path in paths {
        let walker = WalkBuilder::new(path.as_ref())
            .hidden(true) // skip hidden files
            .git_ignore(true)
            .git_global(true)
            .build();

        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path();

            // Check extension
            let ext = match file_path.extension().and_then(|e| e.to_str()) {
                Some(e) => e,
                None => continue,
            };
            if !extensions.iter().any(|allowed| allowed == ext) {
                continue;
            }

            files_scanned += 1;

            // Read file, skip non-UTF-8
            let contents = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: skipping {}: {}", file_path.display(), e);
                    continue;
                }
            };

            let violations = scan_file_contents(&contents, patterns);
            if !violations.is_empty() {
                files.push(FileReport {
                    path: file_path.to_string_lossy().to_string(),
                    owner: None, // filled in later by owners module
                    violations,
                });
            }
        }
    }

    Ok(ScanResult {
        files,
        files_scanned,
    })
}

fn scan_file_contents(contents: &str, patterns: &[CompiledPattern]) -> Vec<Violation> {
    let mut violations = Vec::new();

    for (line_num, line) in contents.lines().enumerate() {
        for pattern in patterns {
            if let Some(caps) = pattern.regex.captures(line) {
                let rules = pattern.extract_rules_from_match(&caps);
                violations.push(Violation {
                    line: line_num + 1,
                    pattern: pattern.name.clone(),
                    category: pattern.category.clone(),
                    rules,
                    raw_text: line.trim().to_string(),
                });
            }
        }
    }

    violations
}
