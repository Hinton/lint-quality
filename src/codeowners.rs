//! CODEOWNERS file parsing and ownership assignment.
//!
//! Supports GitHub CODEOWNERS semantics: glob pattern matching with last-match-wins
//! precedence. Handles anchored/unanchored patterns, directory patterns, and
//! bare names that may refer to either files or directories.

use glob::Pattern;
use std::path::{Path, PathBuf};

/// A single rule from a CODEOWNERS file: glob patterns and their associated owners.
struct Rule {
    /// Primary pattern. For directory-like patterns, a `/**` variant is also stored.
    patterns: Vec<Pattern>,
    owners: Vec<String>,
}

/// Parsed CODEOWNERS file. Rules are stored in order; last match wins (GitHub semantics).
pub struct CodeOwners {
    rules: Vec<Rule>,
    /// Absolute path to the repo root, derived from the CODEOWNERS file location.
    /// File paths are made relative to this before matching.
    repo_root: PathBuf,
}

impl CodeOwners {
    /// Load and parse a CODEOWNERS file. Returns `None` on read errors.
    ///
    /// The repo root is inferred from the file location:
    /// - `.github/CODEOWNERS` → repo root is the grandparent
    /// - `docs/CODEOWNERS` → repo root is the grandparent
    /// - `CODEOWNERS` at repo root → repo root is the parent
    pub fn from_file(path: &Path) -> Option<Self> {
        let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let contents = std::fs::read_to_string(&abs_path)
            .map_err(|e| {
                eprintln!(
                    "Warning: failed to read CODEOWNERS {}: {}",
                    path.display(),
                    e
                );
            })
            .ok()?;

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

    fn parse_rules(contents: &str) -> Vec<Rule> {
        contents
            .lines()
            .map(str::trim)
            // Skip blank lines and comments
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let raw_pattern = parts.next()?;
                let owners: Vec<String> = parts.map(str::to_string).collect();
                if owners.is_empty() {
                    return None;
                }
                let patterns: Vec<Pattern> = codeowners_pattern_to_globs(raw_pattern)
                    .into_iter()
                    .flat_map(|gs| {
                        Pattern::new(&gs).map_err(|_| {
                            eprintln!("Warning: invalid CODEOWNERS pattern: {raw_pattern}")
                        })
                    })
                    .collect();
                (!patterns.is_empty()).then_some(Rule { patterns, owners })
            })
            .collect()
    }

    /// Make a file path relative to the repo root for CODEOWNERS matching.
    fn make_relative(&self, path: &str) -> String {
        Path::new(path)
            .strip_prefix(&self.repo_root)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.strip_prefix("./").unwrap_or(path).to_string())
    }

    /// Find the owners for a given file path. Returns the owners from the last
    /// matching rule (GitHub CODEOWNERS semantics: last match wins).
    fn match_path(&self, relative_path: &str) -> Option<&[String]> {
        self.rules
            .iter()
            .filter(|r| r.patterns.iter().any(|p| p.matches(relative_path)))
            .last()
            .map(|r| r.owners.as_slice())
    }

    /// Look up the primary owner for an absolute or relative file path.
    /// Returns the first owner from the last matching rule, or `None` if unowned.
    pub fn lookup(&self, path: &str) -> Option<String> {
        let relative = self.make_relative(path);
        self.match_path(&relative)
            .and_then(|owners| owners.first())
            .cloned()
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
/// - `client/src/auth` matches the file AND everything under it
///
/// The `glob` crate's `*` only matches within a single directory, so we need
/// to translate patterns to use `**` for cross-directory matching.
///
/// Returns multiple patterns when a bare path (no glob chars, no trailing `/`)
/// could refer to either a file or a directory.
fn codeowners_pattern_to_globs(pattern: &str) -> Vec<String> {
    let clean = pattern.strip_prefix('/').unwrap_or(pattern);
    let (base, is_dir) = clean
        .strip_suffix('/')
        .map_or((clean, false), |s| (s, true));

    if base == "*" {
        return vec!["**".to_string()];
    }

    // Patterns without a leading or internal `/` match at any depth.
    let rooted = pattern.starts_with('/') || base.contains('/');
    let glob_base = if rooted {
        base.to_string()
    } else {
        format!("**/{base}")
    };
    let has_glob = base.contains('*') || base.contains('?');

    if is_dir {
        vec![format!("{glob_base}/**")]
    } else if has_glob {
        vec![glob_base]
    } else {
        // Bare literal: GitHub treats it as matching both a file and a directory prefix.
        vec![glob_base.clone(), format!("{glob_base}/**")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create an Owners with a dummy repo root for unit tests.
    fn parse_test(contents: &str) -> CodeOwners {
        CodeOwners {
            rules: CodeOwners::parse_rules(contents),
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
        // GitHub treats `a/b/src` as matching both the path
        // itself and everything under it.
        let owners = parse_test("a/b/src @team-b\n");
        assert_eq!(owners.match_path("a/b/src").unwrap(), &["@team-b"]);
        assert_eq!(owners.match_path("a/b/src/file.ts").unwrap(), &["@team-b"]);
        assert_eq!(
            owners.match_path("a/b/src/deep/nested.ts").unwrap(),
            &["@team-b"]
        );
        assert!(owners.match_path("a/b/other.ts").is_none());
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
client/native @platform-dev
client/web/src/auth @auth-dev
client/web/src/auth/sso @platform-dev
client/web/src/billing @billing-dev
";
        let owners = parse_test(codeowners);

        // .scss at any depth → @ui-team
        assert_eq!(
            owners
                .match_path("client/web/src/styles/main.scss")
                .unwrap(),
            &["@ui-team"]
        );

        // auth → @auth-dev
        assert_eq!(
            owners.match_path("client/web/src/auth/login.ts").unwrap(),
            &["@auth-dev"]
        );

        // auth/sso → @platform-dev (last match wins)
        assert_eq!(
            owners
                .match_path("client/web/src/auth/sso/proxy.html")
                .unwrap(),
            &["@platform-dev"]
        );

        // billing → @billing-dev
        assert_eq!(
            owners
                .match_path("client/web/src/billing/settings.ts")
                .unwrap(),
            &["@billing-dev"]
        );

        // No match for random ts file
        assert!(owners.match_path("client/web/src/main.ts").is_none());
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
        let owners = CodeOwners {
            rules: vec![],
            repo_root: PathBuf::from("/Users/user/repository"),
        };
        // When canonicalize fails (path doesn't exist on disk), falls back to
        // stripping ./ prefix
        assert_eq!(owners.make_relative("./src/foo.ts"), "src/foo.ts");
        assert_eq!(owners.make_relative("src/foo.ts"), "src/foo.ts");
    }
}
