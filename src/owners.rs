//! CODEOWNERS file parsing and ownership assignment.
//!
//! Supports GitHub CODEOWNERS semantics: glob pattern matching with last-match-wins
//! precedence. Handles anchored/unanchored patterns, directory patterns, and
//! bare names that may refer to either files or directories.

use glob::Pattern;
use std::path::{Path, PathBuf};

use crate::scan::FileReport;

/// A single rule from a CODEOWNERS file: glob patterns and their associated owners.
struct OwnerRule {
    /// Primary pattern. For directory-like patterns, a `/**` variant is also stored.
    patterns: Vec<Pattern>,
    owners: Vec<String>,
}

/// Parsed CODEOWNERS file. Rules are stored in order; last match wins (GitHub semantics).
pub struct Owners {
    rules: Vec<OwnerRule>,
    /// Absolute path to the repo root, derived from the CODEOWNERS file location.
    /// File paths are made relative to this before matching.
    repo_root: PathBuf,
}

impl Owners {
    /// Load and parse a CODEOWNERS file. Returns `None` on read errors.
    ///
    /// The repo root is inferred from the file location:
    /// - `.github/CODEOWNERS` → repo root is the grandparent
    /// - `docs/CODEOWNERS` → repo root is the grandparent
    /// - `CODEOWNERS` at repo root → repo root is the parent
    pub fn from_file(path: &Path) -> Option<Self> {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let contents = match std::fs::read_to_string(&abs_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Warning: failed to read CODEOWNERS {}: {}",
                    path.display(),
                    e
                );
                return None;
            }
        };

        // Derive repo root: CODEOWNERS can live at root, .github/, or docs/
        let parent = abs_path.parent()?;
        let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let repo_root = if parent_name == ".github" || parent_name == "docs" {
            parent.parent()?.to_path_buf()
        } else {
            parent.to_path_buf()
        };

        let rules = Self::parse_rules(&contents);
        Some(Self { rules, repo_root })
    }

    fn parse_rules(contents: &str) -> Vec<OwnerRule> {
        let mut rules = Vec::new();

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let raw_pattern = parts[0];
            let owners: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

            let glob_strs = codeowners_pattern_to_globs(raw_pattern);
            let mut patterns = Vec::new();
            for gs in &glob_strs {
                match Pattern::new(gs) {
                    Ok(p) => patterns.push(p),
                    Err(_) => eprintln!("Warning: invalid CODEOWNERS pattern: {raw_pattern}"),
                }
            }
            if !patterns.is_empty() {
                rules.push(OwnerRule { patterns, owners });
            }
        }

        rules
    }

    /// Make a file path relative to the repo root for CODEOWNERS matching.
    fn make_relative(&self, path: &str) -> String {
        // Try to canonicalize the path, then strip the repo root prefix
        let abs = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(path));

        if let Ok(rel) = abs.strip_prefix(&self.repo_root) {
            rel.to_string_lossy().to_string()
        } else {
            // Fallback: strip common prefixes like ./
            let cleaned = path.strip_prefix("./").unwrap_or(path);
            cleaned.to_string()
        }
    }

    /// Find the owners for a given file path. Returns the owners from the last
    /// matching rule (GitHub CODEOWNERS semantics: last match wins).
    fn match_path(&self, relative_path: &str) -> Option<&[String]> {
        let mut result = None;
        for rule in &self.rules {
            if rule.patterns.iter().any(|p| p.matches(relative_path)) {
                result = Some(rule.owners.as_slice());
            }
        }
        result
    }
}

/// Convert a GitHub CODEOWNERS pattern to one or more glob patterns.
///
/// GitHub CODEOWNERS semantics:
/// - `*` matches everything (any file at any depth)
/// - `*.js` matches any .js file at any depth
/// - `/docs/` matches the `docs/` directory at the repo root
/// - `docs/` matches any `docs/` directory at any depth
/// - `/src/foo.ts` matches exactly `src/foo.ts`
/// - `apps/web/src/connectors` matches the file AND everything under it
///
/// The `glob` crate's `*` only matches within a single directory, so we need
/// to translate patterns to use `**` for cross-directory matching.
///
/// Returns multiple patterns when a bare path (no glob chars, no trailing `/`)
/// could refer to either a file or a directory.
fn codeowners_pattern_to_globs(pattern: &str) -> Vec<String> {
    let anchored = pattern.starts_with('/');
    let clean = pattern.strip_prefix('/').unwrap_or(pattern);

    // Trailing `/` means "match everything inside this directory"
    let (base, is_dir) = if let Some(stripped) = clean.strip_suffix('/') {
        (stripped, true)
    } else {
        (clean, false)
    };

    if is_dir {
        if anchored {
            vec![format!("{base}/**")]
        } else {
            vec![format!("**/{base}/**")]
        }
    } else if base == "*" {
        vec!["**".to_string()]
    } else if !base.contains('*') && !base.contains('?') {
        // No glob chars: could be a file or directory.
        // Emit both the exact match and a directory match (path/**).
        if anchored {
            vec![base.to_string(), format!("{base}/**")]
        } else if base.contains('/') {
            // Has a slash: anchored to root per GitHub docs.
            vec![base.to_string(), format!("{base}/**")]
        } else {
            // Bare name, no slash: matches at any depth.
            vec![format!("**/{base}"), format!("**/{base}/**")]
        }
    } else if anchored {
        vec![base.to_string()]
    } else if !base.contains('/') {
        vec![format!("**/{base}")]
    } else {
        vec![base.to_string()]
    }
}

/// Assign CODEOWNERS to file reports. Paths are made relative to the repo root
/// (derived from the CODEOWNERS file location) before matching.
pub fn assign_owners(files: &mut [FileReport], owners: &Owners) {
    for file in files.iter_mut() {
        let relative = owners.make_relative(&file.path);
        if let Some(owner_list) = owners.match_path(&relative)
            && let Some(first) = owner_list.first()
        {
            file.owner = Some(first.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create an Owners with a dummy repo root for unit tests.
    fn parse_test(contents: &str) -> Owners {
        Owners {
            rules: Owners::parse_rules(contents),
            repo_root: PathBuf::from("/dummy"),
        }
    }

    #[test]
    fn wildcard_matches_everything() {
        let owners = parse_test("* @default\n");
        assert_eq!(owners.match_path("foo.ts").unwrap(), &["@default"]);
        assert_eq!(owners.match_path("src/foo.ts").unwrap(), &["@default"]);
        assert_eq!(owners.match_path("a/b/c/d.ts").unwrap(), &["@default"]);
    }

    #[test]
    fn extension_pattern_matches_at_any_depth() {
        let owners = parse_test("*.vue @frontend\n");
        assert_eq!(owners.match_path("app.vue").unwrap(), &["@frontend"]);
        assert_eq!(owners.match_path("src/app.vue").unwrap(), &["@frontend"]);
        assert_eq!(owners.match_path("a/b/c.vue").unwrap(), &["@frontend"]);
        assert!(owners.match_path("app.ts").is_none());
    }

    #[test]
    fn anchored_directory() {
        let owners = parse_test("/src/ @core\n");
        assert_eq!(owners.match_path("src/foo.ts").unwrap(), &["@core"]);
        assert_eq!(owners.match_path("src/deep/bar.ts").unwrap(), &["@core"]);
        assert!(owners.match_path("lib/src/foo.ts").is_none());
    }

    #[test]
    fn unanchored_directory() {
        let owners = parse_test("docs/ @writers\n");
        assert_eq!(owners.match_path("docs/readme.md").unwrap(), &["@writers"]);
        assert_eq!(owners.match_path("src/docs/api.md").unwrap(), &["@writers"]);
    }

    #[test]
    fn anchored_exact_file() {
        let owners = parse_test("/Makefile @infra\n");
        assert_eq!(owners.match_path("Makefile").unwrap(), &["@infra"]);
        assert!(owners.match_path("src/Makefile").is_none());
    }

    #[test]
    fn unanchored_bare_name() {
        let owners = parse_test("Makefile @infra\n");
        assert_eq!(owners.match_path("Makefile").unwrap(), &["@infra"]);
        assert_eq!(owners.match_path("src/Makefile").unwrap(), &["@infra"]);
    }

    #[test]
    fn last_match_wins() {
        let owners = parse_test("* @default\n*.vue @frontend\n/tests/ @test-team\n");
        assert_eq!(owners.match_path("tests/foo.ts").unwrap(), &["@test-team"]);
        assert_eq!(owners.match_path("tests/app.vue").unwrap(), &["@test-team"]);
        assert_eq!(owners.match_path("src/app.vue").unwrap(), &["@frontend"]);
        assert_eq!(owners.match_path("README.md").unwrap(), &["@default"]);
    }

    #[test]
    fn path_with_slash_is_anchored() {
        let owners = parse_test("src/utils/*.ts @utils-team\n");
        assert_eq!(
            owners.match_path("src/utils/helper.ts").unwrap(),
            &["@utils-team"]
        );
        assert!(owners.match_path("lib/src/utils/helper.ts").is_none());
    }

    #[test]
    fn comments_and_blank_lines_skipped() {
        let owners = parse_test("# comment\n\n* @default\n# another\n*.rs @rust-team\n");
        assert_eq!(owners.match_path("main.rs").unwrap(), &["@rust-team"]);
        assert_eq!(owners.match_path("foo.ts").unwrap(), &["@default"]);
    }

    #[test]
    fn multiple_owners() {
        let owners = parse_test("* @team-a @team-b\n");
        assert_eq!(
            owners.match_path("foo.ts").unwrap(),
            &["@team-a", "@team-b"]
        );
    }

    #[test]
    fn bare_directory_without_trailing_slash() {
        // GitHub treats `apps/web/src/connectors` as matching both the path
        // itself and everything under it.
        let owners = parse_test("apps/web/src/connectors @team-auth\n");
        assert_eq!(
            owners.match_path("apps/web/src/connectors").unwrap(),
            &["@team-auth"]
        );
        assert_eq!(
            owners
                .match_path("apps/web/src/connectors/webauthn.ts")
                .unwrap(),
            &["@team-auth"]
        );
        assert_eq!(
            owners
                .match_path("apps/web/src/connectors/deep/nested.ts")
                .unwrap(),
            &["@team-auth"]
        );
        assert!(owners.match_path("apps/web/src/other.ts").is_none());
    }

    #[test]
    fn anchored_bare_directory_without_trailing_slash() {
        let owners = parse_test("/src @core\n");
        assert_eq!(owners.match_path("src/foo.ts").unwrap(), &["@core"]);
        assert_eq!(owners.match_path("src/deep/bar.ts").unwrap(), &["@core"]);
        assert!(owners.match_path("lib/src/foo.ts").is_none());
    }

    #[test]
    fn bare_name_matches_as_directory_too() {
        // `docs` without slash or glob: matches `docs` itself, `docs/foo.md`,
        // and `src/docs/foo.md` (unanchored).
        let owners = parse_test("docs @writers\n");
        assert_eq!(owners.match_path("docs").unwrap(), &["@writers"]);
        assert_eq!(owners.match_path("docs/foo.md").unwrap(), &["@writers"]);
        assert_eq!(owners.match_path("src/docs/foo.md").unwrap(), &["@writers"]);
    }

    #[test]
    fn real_world_codeowners() {
        let codeowners = "\
*.scss @ui-team
*.css @ui-team
apps/desktop/desktop_native @platform-dev
apps/web/src/connectors @auth-dev
apps/web/src/connectors/platform @platform-dev
apps/web/src/app/billing @billing-dev
";
        let owners = parse_test(codeowners);

        // .scss at any depth → @ui-team
        assert_eq!(
            owners.match_path("apps/web/src/styles/main.scss").unwrap(),
            &["@ui-team"]
        );

        // connectors → @auth-dev
        assert_eq!(
            owners
                .match_path("apps/web/src/connectors/webauthn.ts")
                .unwrap(),
            &["@auth-dev"]
        );

        // connectors/platform → @platform-dev (last match wins)
        assert_eq!(
            owners
                .match_path("apps/web/src/connectors/platform/proxy.html")
                .unwrap(),
            &["@platform-dev"]
        );

        // billing → @billing-dev
        assert_eq!(
            owners
                .match_path("apps/web/src/app/billing/settings.ts")
                .unwrap(),
            &["@billing-dev"]
        );

        // No match for random ts file
        assert!(owners.match_path("apps/web/src/main.ts").is_none());
    }

    #[test]
    fn glob_star_star_behavior() {
        // Verify that `tests/**` actually matches nested paths
        let p = Pattern::new("tests/**").unwrap();
        assert!(
            p.matches("tests/foo.ts"),
            "tests/** should match tests/foo.ts"
        );
        assert!(
            p.matches("tests/fixtures/foo.ts"),
            "tests/** should match tests/fixtures/foo.ts"
        );
    }

    #[test]
    fn make_relative_strips_repo_root() {
        let owners = Owners {
            rules: vec![],
            repo_root: PathBuf::from("/Users/oscar/Code/clients"),
        };
        // When canonicalize fails (path doesn't exist on disk), falls back to
        // stripping ./ prefix
        assert_eq!(owners.make_relative("./src/foo.ts"), "src/foo.ts");
        assert_eq!(owners.make_relative("src/foo.ts"), "src/foo.ts");
    }
}
