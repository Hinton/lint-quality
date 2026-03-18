# lint-quality: Rust CLI for Detecting Lint Suppressions

## Context

Build a Rust CLI tool that scans codebases for disabled lint rules (eslint-disable, ts-ignore, ts-expect-error) and produces reports by directory, rule, violation type, and CODEOWNERS. JSON output supports future web visualization and trend comparison across weekly runs.

The workspace is currently empty (just a placeholder package.json, no git repo).

## Crate Stack

| Purpose | Crate |
|---------|-------|
| CLI | clap v4 (derive) |
| Serialization | serde + serde_json |
| Config | toml (for config file parsing) |
| Regex | regex |
| File walking | ignore (respects .gitignore) |
| CODEOWNERS | codeowners |
| Timestamps | chrono |
| Errors | thiserror + anyhow |

## Project Structure

```
lint-quality/
├── Cargo.toml
├── src/
│   ├── main.rs          # Entry point, orchestration
│   ├── cli.rs           # clap derive structs
│   ├── config.rs        # TOML config loading + merging with CLI args
│   ├── patterns.rs      # Regex definitions + rule extraction
│   ├── scanner.rs       # File walking + line-by-line matching
│   ├── model.rs         # Finding, Report, ReportSummary, ReportMetadata
│   ├── analysis.rs      # Aggregation (by dir, rule, type, owner)
│   ├── owners.rs        # CODEOWNERS integration
│   ├── output/
│   │   ├── mod.rs       # Output trait
│   │   ├── human.rs     # Terminal-friendly report
│   │   └── json.rs      # JSON serialization
│   └── error.rs         # Error types
├── plans/                # Implementation plans
└── tests/
    ├── fixtures/         # Sample files with lint disables
    └── integration.rs
```

## Config File (`lint-quality.toml`)

Auto-discovered in the scanned directory (walks up to repo root), overridable with `--config <path>`. CLI args override config file values.

```toml
# Default output format
format = "human"

# File extensions to scan
extensions = ["js", "jsx", "ts", "tsx", "vue", "svelte", "html", "mjs", "cjs", "mts", "cts"]

# Path to CODEOWNERS file (relative to config file or absolute)
codeowners = ".github/CODEOWNERS"

# Default scan paths (relative to config file)
scan_paths = ["src", "apps", "libs"]

# Built-in patterns are always active unless explicitly disabled
# You can add custom patterns or disable built-in ones

[[patterns]]
name = "eslint-disable-next-line"
regex = '//\s*eslint-disable-next-line(?:\s+(.+))?$'
category = "eslint"
extract_rules = true

[[patterns]]
name = "eslint-disable-line"
regex = '//\s*eslint-disable-line(?:\s+(.+))?$'
category = "eslint"
extract_rules = true

[[patterns]]
name = "eslint-disable-block"
regex = '/\*\s*eslint-disable(?!\s*-)\s*([^*]*?)\s*\*/'
category = "eslint"
extract_rules = true

[[patterns]]
name = "eslint-disable-file"
regex = '/\*\s*eslint-disable(?!\s*-)(?:\s+(.+))?\s*$'
category = "eslint"
extract_rules = true

[[patterns]]
name = "eslint-disable-html"
regex = '<!--\s*eslint-disable(?:-next-line)?(?:\s+([^-][^>]*?))?\s*-->'
category = "eslint"
extract_rules = true

[[patterns]]
name = "ts-ignore"
regex = '//\s*@ts-ignore'
category = "typescript"
extract_rules = false

[[patterns]]
name = "ts-expect-error"
regex = '//\s*@ts-expect-error'
category = "typescript"
extract_rules = false

# To add a custom pattern:
# [[patterns]]
# name = "noqa"
# regex = '#\s*noqa(?::\s*(.+))?$'
# category = "python"
# extract_rules = true

# To disable a built-in pattern:
# [disabled_patterns]
# ts-ignore = true
```

### Config Data Model (`config.rs`)

```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    pub format: Option<String>,
    pub extensions: Option<Vec<String>>,
    pub codeowners: Option<PathBuf>,
    pub scan_paths: Option<Vec<PathBuf>>,
    pub patterns: Option<Vec<PatternConfig>>,
    pub disabled_patterns: Option<HashMap<String, bool>>,
}

#[derive(Debug, Deserialize)]
pub struct PatternConfig {
    pub name: String,
    pub regex: String,
    pub category: String,
    pub extract_rules: bool,
}
```

Config resolution order: defaults -> config file -> CLI args (later wins).

## CLI Interface

```
lint-quality scan [paths...]
  --format [human|json]        # default: human
  --config <path>              # override config file auto-discovery
  --codeowners <path>          # optional CODEOWNERS file
  --extensions js,ts,tsx,...    # override default file extensions
```

Default extensions: `js`, `jsx`, `ts`, `tsx`, `vue`, `svelte`, `html`, `mjs`, `cjs`, `mts`, `cts`

## Data Model & JSON Output

The JSON output is **file-centric**: an array of files, each listing its violations and owner. This makes it natural to drill down by file, directory, or owner.

### Example JSON Output

```json
{
  "metadata": {
    "timestamp": "2026-03-18T10:30:00Z",
    "tool_version": "0.1.0",
    "scanned_paths": ["src", "apps"],
    "files_scanned": 1423,
    "scan_duration_ms": 340
  },
  "files": [
    {
      "path": "src/components/Button.tsx",
      "owner": "@frontend-platform",
      "violations": [
        {
          "line": 12,
          "pattern": "eslint-disable-next-line",
          "category": "eslint",
          "rules": ["@typescript-eslint/no-explicit-any"],
          "raw_text": "// eslint-disable-next-line @typescript-eslint/no-explicit-any"
        },
        {
          "line": 45,
          "pattern": "ts-expect-error",
          "category": "typescript",
          "rules": [],
          "raw_text": "// @ts-expect-error legacy component"
        }
      ]
    },
    {
      "path": "src/legacy/api-client.js",
      "owner": "@payments-team",
      "violations": [
        {
          "line": 1,
          "pattern": "eslint-disable-file",
          "category": "eslint",
          "rules": ["*"],
          "raw_text": "/* eslint-disable"
        },
        {
          "line": 89,
          "pattern": "eslint-disable-next-line",
          "category": "eslint",
          "rules": ["no-unused-vars", "no-undef"],
          "raw_text": "// eslint-disable-next-line no-unused-vars, no-undef"
        }
      ]
    }
  ],
  "summary": {
    "total_violations": 287,
    "total_files_with_violations": 94,
    "by_pattern": {
      "eslint-disable-next-line": 142,
      "ts-expect-error": 63,
      "eslint-disable-file": 41,
      "eslint-disable-block": 18,
      "ts-ignore": 12,
      "eslint-disable-line": 8,
      "eslint-disable-html": 3
    },
    "by_category": {
      "eslint": 212,
      "typescript": 75
    },
    "by_rule": {
      "@typescript-eslint/no-explicit-any": 52,
      "no-unused-vars": 31,
      "react-hooks/exhaustive-deps": 28,
      "*": 41
    },
    "by_directory": {
      "src": 245,
      "src/legacy": 89,
      "src/components": 43,
      "src/utils": 21,
      "apps": 42
    },
    "by_owner": {
      "@frontend-platform": 112,
      "@payments-team": 54,
      "@unowned": 121
    }
  }
}
```

### Key Types in `model.rs`

- **FileReport**: path, owner (optional), violations vec
- **Violation**: line number, pattern name, category, rules vec, raw text
- **Report**: metadata + files (vec of FileReport) + pre-computed summary
- **ReportSummary**: counts by pattern, category, rule, directory (hierarchical), owner

The Report is self-contained with timestamp/metadata so multiple JSON reports can be loaded for trend comparison without schema changes.

## Built-in Detection Patterns

| # | Pattern | Name | Category | Rule Extraction |
|---|---------|------|----------|-----------------|
| 1 | `// eslint-disable-next-line [rules]` | eslint-disable-next-line | eslint | comma-split capture group |
| 2 | `// eslint-disable-line [rules]` | eslint-disable-line | eslint | comma-split capture group |
| 3 | `/* eslint-disable [rules] */` (self-closing) | eslint-disable-block | eslint | comma-split capture group |
| 4 | `/* eslint-disable [rules]` (no closing `*/`) | eslint-disable-file | eslint | comma-split capture group |
| 5 | `<!-- eslint-disable[-next-line] [rules] -->` | eslint-disable-html | eslint | comma-split capture group |
| 6 | `// @ts-ignore` | ts-ignore | typescript | none |
| 7 | `// @ts-expect-error` | ts-expect-error | typescript | none |

When no rules are specified in an eslint-disable, use `["*"]` to represent "all rules disabled".

## Hierarchical Directory Analysis

For each finding, increment counts for every ancestor directory. A finding at `src/components/Button.tsx` increments `src/components/`, `src/`, and root. This supports drill-down in both human output and future web UI.

## Implementation Phases

### Phase 0: Setup
1. Write this plan to `plans/` directory in the repo
2. `git init` + create `.gitignore` (Rust target/, etc.)
3. `cargo init` the project (keep existing package.json)
4. Write `Cargo.toml` with all dependencies (including `toml` crate)

### Phase 1: Core Types & Config
5. Implement `model.rs` - Finding, Report, ReportSummary, ReportMetadata (with dynamic pattern name/category instead of fixed enum)
6. Implement `config.rs` - TOML config parsing, built-in pattern defaults, config file auto-discovery, merging with CLI args
7. Implement `cli.rs` - clap derive structs with `--config` flag
8. Implement `error.rs`
9. Stub `main.rs` that loads config and parses args

### Phase 2: Core Scanning
10. Implement `patterns.rs` - compile regexes from config, `RegexSet` for fast rejection, rule name extraction
11. Implement `scanner.rs` - file walking with `ignore` crate, line-by-line scanning
12. Wire scanner into main

### Phase 3: Analysis & Output
13. Implement `analysis.rs` - build `ReportSummary` from findings
14. Implement `output/human.rs` - formatted terminal report with counts, percentages, top rules/dirs
15. Implement `output/json.rs` - `serde_json::to_string_pretty` on `Report`

### Phase 4: CODEOWNERS
16. Implement `owners.rs` - load and query CODEOWNERS file
17. Integrate into analysis (by-owner aggregation)

### Phase 5: Testing
18. Create test fixtures with known patterns
19. Unit tests for `patterns.rs` (critical - each regex pattern + edge cases)
20. Unit tests for `config.rs` (config loading, merging, auto-discovery)
21. Integration test: scan fixtures, assert counts
22. Test JSON round-trip (serialize -> deserialize -> compare)

## Edge Cases to Handle

- No rules specified in eslint-disable -> `["*"]`
- Rules with slashes (`react-hooks/exhaustive-deps`), `@` scopes, hyphens
- Non-UTF-8 files -> skip with stderr warning
- CODEOWNERS path resolution: use CODEOWNERS parent (or grandparent if in `.github/`) as repo root
- Invalid regex in custom config patterns -> report error at startup, not mid-scan
- Missing config file -> use built-in defaults silently

## Verification

1. `cargo build` compiles cleanly
2. `cargo test` passes all unit + integration tests
3. Run against the lint-quality repo itself (trivial case)
4. Create a test fixture directory and verify human + JSON output
5. Verify JSON output deserializes back to `Report` struct
6. Test with a custom `lint-quality.toml` that adds a pattern and disables one
