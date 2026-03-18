//! Human-readable terminal report with aligned columns, percentage breakdowns, and color.

use crate::model::Report;
use colored::Colorize;
use std::fmt::Write;

/// Format a scan report for terminal display, with sections for patterns,
/// categories, top rules, top directories, and owners.
pub fn format_human(report: &Report) -> String {
    let mut out = String::new();
    let s = &report.summary;

    writeln!(out, "{}", "Lint Quality Report".bold()).unwrap();
    writeln!(out, "{}", "===================".bold()).unwrap();
    if let Some(ref config_path) = report.metadata.config_path {
        writeln!(out, "Using config: {}", config_path).unwrap();
    }
    writeln!(
        out,
        "Scanned {} files, found {} violations in {} files",
        fmt_num(report.metadata.files_scanned),
        fmt_num(s.total_violations).yellow(),
        fmt_num(s.total_files_with_violations).yellow(),
    )
    .unwrap();
    writeln!(
        out,
        "Scan took {}\n",
        fmt_duration(report.metadata.scan_duration_ms)
    )
    .unwrap();

    let total = s.total_violations;

    // By pattern
    writeln!(out, "{}", "By Pattern:".bold()).unwrap();
    let mut patterns: Vec<_> = s.by_pattern.iter().collect();
    patterns.sort_by(|a, b| b.1.cmp(a.1));
    write_section(&mut out, &patterns, total);

    // By category
    writeln!(out, "\n{}", "By Category:".bold()).unwrap();
    let mut cats: Vec<_> = s.by_category.iter().collect();
    cats.sort_by(|a, b| b.1.cmp(a.1));
    write_section(&mut out, &cats, total);

    // Top rules
    writeln!(out, "\n{}", "Top Rules:".bold()).unwrap();
    let mut rules: Vec<_> = s.by_rule.iter().collect();
    rules.sort_by(|a, b| b.1.cmp(a.1));
    rules.truncate(15);
    write_section(&mut out, &rules, total);

    // Top directories
    writeln!(out, "\n{}", "Top Directories:".bold()).unwrap();
    let mut dirs: Vec<_> = s.by_directory.iter().collect();
    dirs.sort_by(|a, b| b.1.cmp(a.1));
    dirs.truncate(15);
    write_section(&mut out, &dirs, total);

    // By owner
    if !s.by_owner.is_empty() {
        writeln!(out, "\n{}", "By Owner:".bold()).unwrap();
        let mut owners: Vec<_> = s.by_owner.iter().collect();
        owners.sort_by(|a, b| b.1.cmp(a.1));
        write_section(&mut out, &owners, total);
    }

    out
}

fn write_section<K: AsRef<str> + std::fmt::Display>(
    out: &mut String,
    items: &[(&K, &usize)],
    total: usize,
) {
    let name_width = items
        .iter()
        .map(|(name, _)| name.as_ref().len())
        .max()
        .unwrap_or(0);
    let count_width = items
        .iter()
        .map(|(_, c)| fmt_num(**c).len())
        .max()
        .unwrap_or(0);
    for (name, count) in items {
        let pct = if total > 0 {
            **count as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let count_str = format!("{:>cw$}", fmt_num(**count), cw = count_width);
        let pct_str = format!("({:>5.1}%)", pct);
        writeln!(
            out,
            "  {:<nw$} {}  {}",
            name,
            count_str.yellow(),
            pct_str.dimmed(),
            nw = name_width,
        )
        .unwrap();
    }
}

fn fmt_num(n: usize) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn fmt_duration(ms: u64) -> String {
    if ms < 1_000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", ms as f64 / 1_000.0)
    }
}
