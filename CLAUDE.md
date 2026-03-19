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

The scan pipeline flows through these modules in order:

1. **cli** — clap derive-based argument parsing. Two subcommands: `scan` and `read`.
2. **config** — Loads `lint-quality.toml` (auto-discovered by walking up directories or explicit `--config`). Merges defaults → config file → CLI overrides into `ResolvedConfig`. Patterns are config-file-only.
3. **patterns** — Compiles `PatternConfig` regex strings into `CompiledPattern` structs with rule extraction logic.
4. **scanner** — Walks directories with the `ignore` crate (respects `.gitignore`), filters by extension, tests each line against compiled patterns, produces `ScanResult`.
5. **owners** — Parses CODEOWNERS files and assigns owners to file reports (last-match-wins semantics).
6. **report** — Builds the full `Report` with metadata and pre-computed summaries (by_pattern, by_category, by_rule, by_directory, by_owner).
7. **output** — Three formatters: `human` (colored terminal), `json` (serialized Report), `tui` (ratatui interactive browser).

Core data types live in **model** (`Violation`, `FileReport`, `ScanResult`, `Report`, `ReportSummary`).

The `read` subcommand deserializes a previously saved JSON report and re-renders it in a chosen format.
