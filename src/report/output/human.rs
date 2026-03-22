//! Human-readable terminal report with aligned columns, percentage breakdowns, and color.

use super::fmt_num;
use crate::report::Report;
use colored::Colorize;
use std::fmt::Write;

/// Sort a map's entries by count descending, keeping at most `limit` entries.
fn sorted_entries(
    map: &std::collections::HashMap<String, usize>,
    limit: usize,
) -> Vec<(&String, &usize)> {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));
    items.truncate(limit);
    items
}

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

    writeln!(out, "{}", "By Pattern:".bold()).unwrap();
    write_section(&mut out, &sorted_entries(&s.by_pattern, usize::MAX), total);

    writeln!(out, "\n{}", "By Category:".bold()).unwrap();
    write_section(&mut out, &sorted_entries(&s.by_category, usize::MAX), total);

    writeln!(out, "\n{}", "Top Rules:".bold()).unwrap();
    write_section(&mut out, &sorted_entries(&s.by_rule, 15), total);

    writeln!(out, "\n{}", "Top Directories:".bold()).unwrap();
    write_section(&mut out, &sorted_entries(&s.by_directory, 15), total);

    if !s.by_owner.is_empty() {
        writeln!(out, "\n{}", "By Owner:".bold()).unwrap();
        write_section(&mut out, &sorted_entries(&s.by_owner, usize::MAX), total);
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
    // Items are sorted descending, so the first entry has the largest count and widest string.
    let count_width = items
        .first()
        .map(|(_, c)| fmt_num(**c).len())
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

fn fmt_duration(ms: u64) -> String {
    if ms < 1_000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", ms as f64 / 1_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_under_1s() {
        assert_eq!(fmt_duration(0), "0ms");
        assert_eq!(fmt_duration(999), "999ms");
    }

    #[test]
    fn duration_exactly_1s() {
        assert_eq!(fmt_duration(1_000), "1.00s");
    }

    #[test]
    fn duration_fractional_seconds() {
        assert_eq!(fmt_duration(1_500), "1.50s");
        assert_eq!(fmt_duration(12_345), "12.35s");
    }
}
