//! JSON report output via serde serialization.

use crate::report::Report;
use anyhow::Result;

/// Serialize the report to pretty-printed JSON.
pub fn format_json(report: &Report) -> Result<String> {
    Ok(serde_json::to_string_pretty(report)?)
}
