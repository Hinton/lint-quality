use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::report::Report;

/// Load reports from a list of paths. Each path can be a JSON file or a directory
/// containing JSON files. Returns reports sorted by timestamp (oldest first).
pub fn load_reports(paths: &[PathBuf]) -> Result<Vec<Report>> {
    let json_files: Vec<PathBuf> = paths
        .iter()
        .map(|path| -> Result<Vec<PathBuf>> {
            if path.is_dir() {
                std::fs::read_dir(path)?
                    .map(|e| Ok(e?.path()))
                    .filter(|r| {
                        r.as_ref().map_or(true, |p| {
                            p.extension().and_then(|e| e.to_str()) == Some("json")
                        })
                    })
                    .collect()
            } else if path.is_file() {
                Ok(vec![path.clone()])
            } else {
                bail!("path does not exist: {}", path.display())
            }
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    if json_files.is_empty() {
        bail!(
            "no JSON report files found in: {}",
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let (reports, errors): (Vec<_>, Vec<_>) = json_files
        .iter()
        .map(|f| load_report(f).map_err(|e| format!("{}: {}", f.display(), e)))
        .partition(Result::is_ok);

    let mut reports: Vec<Report> = reports.into_iter().map(Result::unwrap).collect();
    let errors: Vec<String> = errors.into_iter().map(Result::unwrap_err).collect();

    if reports.is_empty() {
        bail!("failed to load any valid reports:\n{}", errors.join("\n"));
    }

    if !errors.is_empty() {
        eprintln!(
            "warning: skipped {} invalid file(s):\n{}",
            errors.len(),
            errors.join("\n")
        );
    }

    reports.sort_by(|a, b| a.metadata.timestamp.cmp(&b.metadata.timestamp));
    Ok(reports)
}

fn load_report(path: &Path) -> Result<Report> {
    let contents = std::fs::read_to_string(path)?;
    let report: Report = serde_json::from_str(&contents)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_report_json(timestamp: &str) -> String {
        format!(
            r#"{{
  "metadata": {{
    "timestamp": "{}",
    "tool_version": "0.1.0",
    "scanned_paths": ["."],
    "files_scanned": 1,
    "scan_duration_ms": 1
  }},
  "files": [],
  "summary": {{
    "total_violations": 0,
    "total_files_with_violations": 0,
    "by_pattern": {{}},
    "by_category": {{}},
    "by_rule": {{}},
    "by_directory": {{}},
    "by_owner": {{}}
  }}
}}"#,
            timestamp
        )
    }

    #[test]
    fn load_from_directory() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("a.json"),
            make_report_json("2026-03-01T00:00:00Z"),
        )
        .unwrap();
        fs::write(
            dir.path().join("b.json"),
            make_report_json("2026-02-01T00:00:00Z"),
        )
        .unwrap();
        fs::write(dir.path().join("not-json.txt"), "hello").unwrap();

        let reports = load_reports(&[dir.path().to_path_buf()]).unwrap();
        assert_eq!(reports.len(), 2);
        // Should be sorted oldest first
        assert!(reports[0].metadata.timestamp < reports[1].metadata.timestamp);
    }

    #[test]
    fn load_no_reports_is_error() {
        let dir = TempDir::new().unwrap();
        let result = load_reports(&[dir.path().to_path_buf()]);
        assert!(result.is_err());
    }
}
