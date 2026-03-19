//! Regex pattern compilation and lint rule extraction from matches.

use anyhow::{Context, Result};
use regex::Regex;

use crate::config::PatternConfig;

/// A detection pattern compiled from config, ready to match against source lines.
pub struct CompiledPattern {
    pub name: String,
    pub category: String,
    pub regex: Regex,
    /// When true, capture group 1 is parsed as a comma-separated list of rule names.
    pub extract_rules: bool,
}

impl CompiledPattern {
    /// Extract rules from a regex match. Returns `["*"]` if extract_rules is true
    /// but no rules are captured (meaning "all rules disabled").
    pub fn extract_rules_from_match(&self, caps: &regex::Captures) -> Vec<String> {
        if !self.extract_rules {
            return vec![];
        }
        match caps.get(1) {
            Some(m) => {
                let text = m.as_str().trim();
                if text.is_empty() {
                    vec!["*".to_string()]
                } else {
                    text.split(',')
                        .map(|r| r.trim().to_string())
                        .filter(|r| !r.is_empty())
                        .collect()
                }
            }
            None => vec!["*".to_string()],
        }
    }
}

/// Compile all configured patterns into ready-to-match regexes.
pub fn compile_patterns(configs: &[PatternConfig]) -> Result<Vec<CompiledPattern>> {
    configs
        .iter()
        .map(|pc| {
            let regex = Regex::new(&pc.regex)
                .with_context(|| format!("invalid regex pattern '{}': {}", pc.name, pc.regex))?;
            Ok(CompiledPattern {
                name: pc.name.clone(),
                category: pc.category.clone(),
                regex,
                extract_rules: pc.extract_rules,
            })
        })
        .collect()
}
