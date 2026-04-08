use anyhow::{Context, Result};
use std::path::Path;

use super::server::Assets;
use crate::report::Report;

/// Export the trend dashboard as a self-contained static site.
///
/// Writes `index.html` (with embedded report data) and all bundled assets
/// into `out_dir`, creating the directory tree as needed.
pub fn export(reports: &[Report], out_dir: &Path) -> Result<()> {
    let reports_json = serde_json::to_string(reports)?;

    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create export directory: {}", out_dir.display()))?;

    // Write index.html with injected reports
    let index_bytes = Assets::get("index.html").context("index.html not found in embedded assets")?;
    let html = String::from_utf8_lossy(&index_bytes.data);
    let injected = html.replace(
        "</head>",
        &format!(
            "<script>window.__REPORTS__={}</script></head>",
            reports_json
        ),
    );
    // Rewrite absolute asset paths to relative so the page works via file://
    let injected = injected.replace("src=\"/assets/", "src=\"assets/");
    let injected = injected.replace("href=\"/assets/", "href=\"assets/");
    std::fs::write(out_dir.join("index.html"), injected)?;

    // Write all other embedded assets
    for path in Assets::iter() {
        if path.as_ref() == "index.html" {
            continue;
        }
        let asset = Assets::get(path.as_ref()).unwrap();
        let dest = out_dir.join(path.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, &asset.data)?;
    }

    let index_path = out_dir.join("index.html");
    println!("{}", index_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_report(timestamp: &str) -> Report {
        let json = format!(
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
        );
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn export_creates_index_html() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("out");
        let reports = vec![make_report("2026-03-01T00:00:00Z")];

        export(&reports, &out).unwrap();

        let index = out.join("index.html");
        assert!(index.exists(), "index.html should exist");
        let content = std::fs::read_to_string(&index).unwrap();
        assert!(
            content.contains("window.__REPORTS__="),
            "should inject reports"
        );
        // Asset paths should be relative for file:// access
        assert!(
            !content.contains("src=\"/assets/") && !content.contains("href=\"/assets/"),
            "asset paths should be relative, not absolute"
        );
    }

    #[test]
    fn export_creates_nested_output_dir() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("a").join("b").join("c");
        let reports = vec![make_report("2026-03-01T00:00:00Z")];

        export(&reports, &out).unwrap();
        assert!(out.join("index.html").exists());
    }

    #[test]
    fn export_writes_all_assets() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("out");
        let reports = vec![make_report("2026-03-01T00:00:00Z")];

        export(&reports, &out).unwrap();

        // Every embedded asset should be present on disk
        for path in Assets::iter() {
            let dest = out.join(path.as_ref());
            assert!(dest.exists(), "asset missing: {}", path.as_ref());
        }
    }
}
