# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

lint-quality is a Rust CLI tool that detects disabled lint rules (eslint-disable, @ts-ignore, noqa, etc.) in codebases. Users define regex patterns in `lint-quality.toml`; the tool scans files and produces reports by file, directory, rule, and CODEOWNERS owner.

## Build & Test

```sh
cargo build                    # debug build
cargo build --release          # release build
cargo test                     # all tests (unit + integration)
cargo test <test_name>         # single test, e.g. cargo test scan_fixtures_json
```

Rust edition 2024. Integration tests are in `tests/integration.rs` and use test fixtures in `tests/fixtures/`.

## Architecture

Domain-driven module structure. No `mod.rs` files — use the `name.rs` + `name/` pattern instead.

- **cli** — clap derive-based argument parsing. Subcommands: `scan`, `read`, `trend`.
- **config** (`config.rs`) — Loads `lint-quality.toml` (auto-discovered by walking up directories or explicit `--config`). Merges defaults → config file → CLI overrides into `ResolvedConfig`. Patterns are config-file-only.
- **scan** (`scan.rs` + `scan/`) — Scan domain. Owns `Violation`, `FileReport`, `ScanResult` types.
  - `scan/patterns.rs` — Compiles `PatternConfig` regex strings into `CompiledPattern` with rule extraction.
  - `scan/scanner.rs` — Walks directories with the `ignore` crate (respects `.gitignore`), filters by extension, matches lines.
  - `scan/model.rs` — Scan domain types.
- **owners** (`owners.rs`) — Parses CODEOWNERS files and assigns owners to file reports (last-match-wins semantics).
- **report** (`report.rs` + `report/`) — Report domain. Owns `Report`, `ReportMetadata`, `ReportSummary` types, plus `build()` and `print()`.
  - `report/analysis.rs` — Aggregates violations into summary counts by multiple dimensions.
  - `report/output.rs` + `report/output/` — Three formatters: `human` (colored terminal), `json` (serialized Report), `tui` (ratatui interactive browser).
- **trend** — Loads multiple JSON reports and serves a trend dashboard.

The `read` subcommand deserializes a previously saved JSON report and re-renders it in a chosen format.

## Conventions

- **TypeScript file naming**: Use kebab-case (dash-separated words), not camelCase. E.g. `api-client.ts`, not `apiClient.ts`.
