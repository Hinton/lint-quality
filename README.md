# lint-quality

Detect disabled lint rules in codebases. Define regex patterns in a config file to scan for any type
of lint suppression — eslint-disable comments, `@ts-ignore`, `noqa`, `rubocop:disable`, or anything
else.

Produces reports by file, directory, rule, violation type, and CODEOWNERS. JSON output supports
trend comparison across runs.

## Usage

```sh
lint-quality scan [paths...] [options]
```

### Options

| Flag                        | Description                                    |
| --------------------------- | ---------------------------------------------- |
| `--format human\|json\|tui` | Output format (default: `human`)               |
| `--config <path>`           | Path to config file (overrides auto-discovery) |
| `--codeowners <path>`       | Path to CODEOWNERS file                        |
| `--extensions js,ts,...`    | File extensions to scan (comma-separated)      |

### Examples

```sh
# Scan current directory using auto-discovered config
lint-quality scan .

# Scan specific directories with JSON output
lint-quality scan src apps --format json

# Use a specific config file
lint-quality scan --config ./lint-quality.toml src
```

### TUI output

```sh
lint-quality scan src --format tui
```

Or re-open a saved JSON report in the TUI:

```sh
lint-quality read report.json --format tui
```

The TUI has two panels:

- **Left — Filters**: Browse and toggle filters by Pattern, Category, Rule, or Owner. Use `←`/`→` to
  switch filter dimensions; `Space`/`Enter` to toggle a value; `c` to clear all filters.
- **Right — Data**: View violations aggregated by Files, Rules, Patterns, Categories, Owners, or
  Directories. Use `←`/`→` to switch views. In the Directories view, `Space`/`Enter` expands or
  collapses a directory.

Press `Tab` to move focus between panels. Navigate rows with `↑`/`↓` (or `j`/`k`). Press `q` or
`Esc` to quit.

## Configuration

Create a `lint-quality.toml` in your project root. The tool auto-discovers this file by walking up
from the scanned directory.

### Starter config for JavaScript/TypeScript

Copy this into your `lint-quality.toml` and adjust as needed:

```toml
# Output format: "human" or "json"
format = "human"

# File extensions to scan
extensions = ["js", "jsx", "ts", "tsx", "html", "mjs", "cjs"]

# Path to CODEOWNERS file (optional)
codeowners = ".github/CODEOWNERS"

# Directories to scan when no paths are given on the command line
scan_paths = ["src", "apps", "libs"]

# // eslint-disable-next-line [rules]
[[patterns]]
name = "eslint-disable-next-line"
regex = '//\s*eslint-disable-next-line(?:\s+(.+))?$'
category = "eslint"
extract_rules = true

# // eslint-disable-line [rules]
[[patterns]]
name = "eslint-disable-line"
regex = '//\s*eslint-disable-line(?:\s+(.+))?$'
category = "eslint"
extract_rules = true

# /* eslint-disable rules */ (self-closing block)
[[patterns]]
name = "eslint-disable-block"
regex = '/\*\s*eslint-disable\s+([^*]*?)\s*\*/'
category = "eslint"
extract_rules = true

# /* eslint-disable */ (all rules, self-closing)
[[patterns]]
name = "eslint-disable-block-all"
regex = '/\*\s*eslint-disable\s*\*/'
category = "eslint"
extract_rules = true

# /* eslint-disable (file-level, no closing */)
[[patterns]]
name = "eslint-disable-file"
regex = '/\*\s*eslint-disable\s*$'
category = "eslint"
extract_rules = true

# /* eslint-disable rules (file-level with rules, no closing */)
[[patterns]]
name = "eslint-disable-file-rules"
regex = '^/\*\s*eslint-disable\s+([^*]+?)\s*$'
category = "eslint"
extract_rules = true

# <!-- eslint-disable[-next-line] [rules] --> (HTML/Vue templates)
[[patterns]]
name = "eslint-disable-html"
regex = '<!--\s*eslint-disable(?:-next-line)?(?:\s+([^-][^>]*?))?\s*-->'
category = "eslint"
extract_rules = true

# // @ts-ignore
[[patterns]]
name = "ts-ignore"
regex = '//\s*@ts-ignore'
category = "typescript"
extract_rules = false

# // @ts-expect-error
[[patterns]]
name = "ts-expect-error"
regex = '//\s*@ts-expect-error'
category = "typescript"
extract_rules = false
```

### Other languages

The same pattern format works for any language:

```toml
# Python noqa
[[patterns]]
name = "noqa"
regex = '#\s*noqa(?::\s*(.+))?$'
category = "python"
extract_rules = true

# Ruby rubocop:disable
[[patterns]]
name = "rubocop-disable"
regex = '#\s*rubocop:disable\s+(.+)$'
category = "rubocop"
extract_rules = true
```

### Pattern fields

| Field           | Description                                                                                                                                                |
| --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `name`          | Unique identifier for this pattern                                                                                                                         |
| `regex`         | Regex matched against each line. Capture group 1 (if present) holds the rule list                                                                          |
| `category`      | Grouping for summary (e.g. "eslint", "typescript")                                                                                                         |
| `extract_rules` | When `true`, capture group 1 is split on commas to extract individual rule names. When no rules are captured, `*` is used (meaning "all rules suppressed") |

### Config resolution

The config file is auto-discovered by walking up from the scan directory, or specified explicitly
with `--config`. CLI args override config file values.

## Trend dashboard

Compare reports over time with an interactive web dashboard.

```sh
# Launch dashboard from a directory of JSON reports
lint-quality trend reports/

# Launch from specific report files
lint-quality trend report-jan.json report-feb.json report-mar.json

# Custom port, don't auto-open browser
lint-quality trend reports/ --port 9000 --no-open
```

The dashboard shows all dimensions (total, owner, category, rule, pattern, directory) on a single
page with:

- **Trend charts** — violation counts over time for each dimension
- **Summary tables** — first vs. latest report comparison with delta and percent change
- **Directory tree** — collapsible hierarchical view of violations by directory
- **Filter builder** — add cross-dimensional filters (e.g. filter by owner + rule simultaneously)
- **Insights** — auto-generated observations about biggest improvements and regressions
- **Drag & drop** — upload additional JSON reports directly in the browser

### Generating reports for trending

Save JSON reports from each scan run, e.g. in CI:

```sh
lint-quality scan src --format json > reports/$(date +%Y-%m-%d).json
```

Then visualize the trend:

```sh
lint-quality trend reports/
```

## Reading saved reports

Re-render a previously saved JSON report:

```sh
# Re-render as human-readable output
lint-quality read report.json

# Read from stdin
cat report.json | lint-quality read -
```

## JSON output

The JSON output is file-centric with pre-computed summaries:

```json
{
  "metadata": {
    "timestamp": "2026-03-18T10:30:00Z",
    "tool_version": "0.1.0",
    "scanned_paths": ["src"],
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
        }
      ]
    }
  ],
  "summary": {
    "total_violations": 287,
    "total_files_with_violations": 94,
    "by_pattern": { "eslint-disable-next-line": 142 },
    "by_category": { "eslint": 212, "typescript": 75 },
    "by_rule": { "@typescript-eslint/no-explicit-any": 52 },
    "by_directory": { "src": 245, "src/components": 43 },
    "by_owner": { "@frontend-platform": 112, "@unowned": 121 }
  }
}
```

## Building

```sh
cargo build --release
```
