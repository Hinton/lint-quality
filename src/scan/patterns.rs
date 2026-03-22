//! Regex pattern compilation and lint rule extraction from matches.

use anyhow::{Context, Result};
use regex::Regex;

use crate::config::PatternConfig;

/// A detection pattern compiled from config, ready to match against source lines.
#[derive(Debug)]
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
        let text = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if text.is_empty() {
            return vec!["*".to_string()];
        }
        text.split(',')
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(str::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn pattern(regex: &str, extract_rules: bool) -> CompiledPattern {
        CompiledPattern {
            name: "test".to_string(),
            category: "test".to_string(),
            regex: Regex::new(regex).unwrap(),
            extract_rules,
        }
    }

    fn captures<'a>(p: &'a CompiledPattern, input: &'a str) -> regex::Captures<'a> {
        p.regex.captures(input).unwrap()
    }

    // --- extract_rules_from_match ---

    #[test]
    fn extract_rules_disabled_returns_empty() {
        let p = pattern(r"@ts-ignore", false);
        let caps = captures(&p, "@ts-ignore");
        assert!(p.extract_rules_from_match(&caps).is_empty());
    }

    #[test]
    fn no_capture_group_returns_wildcard() {
        let p = pattern(r"@ts-ignore", true);
        let caps = captures(&p, "@ts-ignore");
        assert_eq!(p.extract_rules_from_match(&caps), vec!["*"]);
    }

    #[test]
    fn empty_capture_group_returns_wildcard() {
        let p = pattern(r"eslint-disable(.*)", true);
        let caps = captures(&p, "eslint-disable");
        assert_eq!(p.extract_rules_from_match(&caps), vec!["*"]);
    }

    #[test]
    fn whitespace_only_capture_returns_wildcard() {
        let p = pattern(r"eslint-disable(.*)", true);
        let caps = captures(&p, "eslint-disable   ");
        assert_eq!(p.extract_rules_from_match(&caps), vec!["*"]);
    }

    #[test]
    fn single_rule_extracted() {
        let p = pattern(r"eslint-disable-next-line\s+(.*)", true);
        let caps = captures(&p, "eslint-disable-next-line no-console");
        assert_eq!(p.extract_rules_from_match(&caps), vec!["no-console"]);
    }

    #[test]
    fn multiple_rules_extracted() {
        let p = pattern(r"eslint-disable-next-line\s+(.*)", true);
        let caps = captures(&p, "eslint-disable-next-line no-console, no-unused-vars");
        assert_eq!(
            p.extract_rules_from_match(&caps),
            vec!["no-console", "no-unused-vars"]
        );
    }

    #[test]
    fn rules_are_trimmed() {
        let p = pattern(r"eslint-disable-next-line\s+(.*)", true);
        let caps = captures(&p, "eslint-disable-next-line  no-console ,  no-unused-vars  ");
        assert_eq!(
            p.extract_rules_from_match(&caps),
            vec!["no-console", "no-unused-vars"]
        );
    }

    #[test]
    fn empty_segments_filtered() {
        let p = pattern(r"eslint-disable-next-line\s+(.*)", true);
        let caps = captures(&p, "eslint-disable-next-line no-console,,no-unused-vars");
        assert_eq!(
            p.extract_rules_from_match(&caps),
            vec!["no-console", "no-unused-vars"]
        );
    }

    // --- compile_patterns ---

    #[test]
    fn valid_pattern_compiles() {
        let configs = vec![PatternConfig {
            name: "ts-ignore".to_string(),
            category: "typescript".to_string(),
            regex: r"@ts-ignore".to_string(),
            extract_rules: false,
        }];
        let compiled = compile_patterns(&configs).unwrap();
        assert_eq!(compiled.len(), 1);
        assert_eq!(compiled[0].name, "ts-ignore");
    }

    #[test]
    fn invalid_regex_returns_error_with_pattern_name() {
        let configs = vec![PatternConfig {
            name: "broken".to_string(),
            category: "test".to_string(),
            regex: r"[invalid".to_string(),
            extract_rules: false,
        }];
        let err = compile_patterns(&configs).unwrap_err();
        assert!(err.to_string().contains("broken"));
    }

    #[test]
    fn empty_configs_returns_empty_vec() {
        assert!(compile_patterns(&[]).unwrap().is_empty());
    }
}

/// Compile all configured patterns into ready-to-match regexes.
pub fn compile_patterns(configs: &[PatternConfig]) -> Result<Vec<CompiledPattern>> {
    configs
        .iter()
        .map(|pc| {
            let regex = Regex::new(&pc.regex)
                .with_context(|| format!("invalid regex for pattern '{}'", pc.name))?;
            Ok(CompiledPattern {
                name: pc.name.clone(),
                category: pc.category.clone(),
                regex,
                extract_rules: pc.extract_rules,
            })
        })
        .collect()
}
