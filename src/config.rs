//! Configuration loading, auto-discovery, and resolution.
//!
//! Supports two layers of configuration (last wins):
//! 1. Built-in defaults (format, extensions)
//! 2. `lint-quality.toml` config file
//!
//! The CLI can override `format`, `extensions`, `codeowners`, and `scan_paths`;
//! patterns are config-file-only.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Load the config file, either from an explicit path or by auto-discovering
/// `lint-quality.toml` in parent directories of the scan target.
pub fn load_config(
    config_path: &Option<PathBuf>,
    scan_paths: &[PathBuf],
) -> Result<(Option<ConfigFile>, Option<String>)> {
    if let Some(p) = config_path {
        return Ok((
            Some(load_config_file(p)?),
            Some(p.to_string_lossy().into_owned()),
        ));
    }
    let start = scan_paths.first().map_or(Path::new("."), PathBuf::as_path);
    let Some(p) = discover_config(start) else {
        return Ok((None, None));
    };
    let display = p.to_string_lossy().into_owned();
    Ok((load_config_file(&p).ok(), Some(display)))
}

/// Raw deserialized content of a `lint-quality.toml` config file.
/// All fields are optional except `patterns` — without patterns, nothing will be detected.
#[derive(Debug, Deserialize, Default)]
pub struct ConfigFile {
    pub format: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub codeowners: Option<PathBuf>,
    pub scan_paths: Option<Vec<PathBuf>>,
    pub patterns: Option<Vec<PatternConfig>>,
}

/// Configuration for a single detection pattern.
#[derive(Debug, Clone, Deserialize)]
pub struct PatternConfig {
    /// Unique identifier for this pattern (e.g. "eslint-disable-next-line").
    pub name: String,
    /// Regex to match against each line. Capture group 1, if present, holds the rule list.
    pub regex: String,
    /// Category grouping (e.g. "eslint", "typescript").
    pub category: String,
    /// Whether to extract individual rule names from the regex capture group.
    pub extract_rules: bool,
}

/// Resolved configuration after merging defaults, config file, and CLI args.
#[derive(Debug)]
pub struct ResolvedConfig {
    pub format: String,
    pub extensions: Vec<String>,
    pub codeowners: Option<PathBuf>,
    pub scan_paths: Vec<PathBuf>,
    pub patterns: Vec<PatternConfig>,
}

/// Returns the default set of file extensions to scan (JS/TS ecosystem).
pub fn default_extensions() -> Vec<String> {
    ["js", "jsx", "ts", "tsx", "html", "mjs", "cjs"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Walk up from `start` looking for `lint-quality.toml`.
pub fn discover_config(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?
    } else {
        start
    }
    .to_path_buf();
    loop {
        let candidate = dir.join("lint-quality.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

pub fn load_config_file(path: &Path) -> Result<ConfigFile> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading config file {}", path.display()))?;
    let config: ConfigFile = toml::from_str(&contents)
        .with_context(|| format!("parsing config file {}", path.display()))?;
    Ok(config)
}

/// Merge defaults, config file, and CLI overrides into a resolved config.
pub fn resolve_config(
    config_file: Option<ConfigFile>,
    cli_format: Option<&str>,
    cli_extensions: Option<&[String]>,
    cli_codeowners: Option<&Path>,
    cli_scan_paths: Vec<PathBuf>,
) -> ResolvedConfig {
    let cf = config_file.unwrap_or_default();

    let format = cli_format
        .map(String::from)
        .or(cf.format)
        .unwrap_or_else(|| "human".into());

    let extensions = cli_extensions
        .map(|e| e.to_vec())
        .or(cf.extensions)
        .unwrap_or_else(default_extensions);

    let codeowners = cli_codeowners.map(PathBuf::from).or(cf.codeowners);

    let scan_paths = if cli_scan_paths.is_empty() {
        cf.scan_paths.unwrap_or_else(|| vec![PathBuf::from(".")])
    } else {
        cli_scan_paths
    };

    let patterns = cf.patterns.unwrap_or_default();

    ResolvedConfig {
        format,
        extensions,
        codeowners,
        scan_paths,
        patterns,
    }
}
