# Plan: Web-based Trend Analysis Dashboard

## Context

lint-quality produces JSON reports with violation counts broken down by pattern, category, rule, directory, and codeowner. Users run scans over time and want to see whether lint ignores are trending up or down. Currently there's no way to compare reports across time. This change adds a `trend` subcommand that launches a local web dashboard for multi-report trend analysis.

## Architecture

New subcommand: `lint-quality trend ./reports/` (or explicit files).

```
Browser (Vue SPA)                     Rust (axum)
+---------------------------+         +---------------------------+
| Vue 3 + Chart.js          | <-----> | GET /api/reports          |
| Tailwind CSS              |   JSON  |   -> Vec<Report> as JSON  |
| Trend composables         |         |                           |
| Filter/upload components  |         | POST /api/reports         |
+---------------------------+         |   -> accept uploads       |
     Built by Vite                    | GET /* -> embedded dist/  |
     Embedded via rust-embed          +---------------------------+
```

- **Rust side is thin**: load files from disk, serialize as JSON, serve Vite build output
- **Vue side does all trend computation**: reports are small (~50KB each), 100+ reports work fine client-side
- **All frontend assets embedded in binary** via rust-embed (fully offline)
- **build.rs** auto-runs `npm run build` in `web/` during `cargo build`

## New Rust Dependencies (Cargo.toml)

```toml
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal"] }
open = "5"
rust-embed = "8"
mime_guess = "2"
```

Tokio runtime is created on-demand only when `trend` subcommand runs (not for `scan`/`read`).

## Frontend Stack (web/)

```json
// key dependencies in web/package.json
{
  "vue": "^3.5",
  "chart.js": "^4",
  "vue-chartjs": "^5",       // Vue 3 Chart.js wrapper
  "vite": "^6",
  "@vitejs/plugin-vue": "^5",
  "tailwindcss": "^4",
  "@tailwindcss/vite": "^4"
}
```

## Files to Create

### Rust
- `src/trend/mod.rs` — entry point: load reports, build runtime, start server, open browser
- `src/trend/server.rs` — axum router, routes, embedded asset serving
- `src/trend/loader.rs` — find `*.json` in dirs, deserialize `Vec<Report>`, sort by timestamp
- `build.rs` — runs `npm install && npm run build` in `web/` before cargo compiles

### Vue Frontend (`web/`)
- `web/package.json` — dependencies
- `web/vite.config.ts` — Vite config, output to `web/dist/`
- `web/index.html` — Vite entry point
- `web/src/main.ts` — Vue app bootstrap
- `web/src/App.vue` — root component with tab navigation
- `web/src/types.ts` — TypeScript interfaces matching Rust Report/Summary types
- `web/src/composables/useTrends.ts` — trend computation logic (series extraction, insights)
- `web/src/composables/useReports.ts` — fetch reports from API, manage uploads
- `web/src/components/TrendChart.vue` — Chart.js line chart wrapper
- `web/src/components/SummaryTable.vue` — first/last/delta table
- `web/src/components/InsightsPanel.vue` — auto-generated insight bullets
- `web/src/components/FilterPanel.vue` — checkbox filters for dimension values
- `web/src/components/DimensionTabs.vue` — Total / Owner / Category / Rule / Directory tabs

## Files to Modify

- `Cargo.toml` — add new dependencies
- `src/cli.rs` — add `Trend` variant to `Commands` enum
- `src/main.rs` — add match arm for `Commands::Trend`, add `mod trend`

## CLI Interface

```rust
// In cli.rs Commands enum
Trend {
    #[arg(required = true)]
    paths: Vec<PathBuf>,       // directories or individual JSON files
    #[arg(long, default_value = "8080")]
    port: u16,
    #[arg(long)]
    no_open: bool,             // skip auto-opening browser
}
```

## API Endpoints

| Route | Response |
|-------|----------|
| `GET /api/reports` | `Vec<Report>` as JSON (pre-serialized at startup) |
| `POST /api/reports` | Accept uploaded JSON report(s) — for drag-and-drop |
| `GET /` | `index.html` from embedded dist/ |
| `GET /*` | Embedded static assets from Vite build |

## Frontend Dashboard Layout

```
+------------------------------------------------------------------+
|  lint-quality trends            [N reports, date range]           |
+------------------------------------------------------------------+
|  View: [Total] [By Owner] [By Category] [By Rule] [By Directory] |
+------------------------------------------------------------------+
|                                                                   |
|  +-------------------------------------------------------------+ |
|  |                    LINE CHART                                | |
|  |  Y: violation count    X: report timestamp                   | |
|  |  One line per selected dimension value                       | |
|  +-------------------------------------------------------------+ |
|                                                                   |
|  +---------------------------+  +-----------------------------+   |
|  | Filter checkboxes         |  | Summary table               |   |
|  | (toggle lines on/off)     |  | Name | First | Last | Delta |   |
|  +---------------------------+  +-----------------------------+   |
|                                                                   |
|  Insights:                                                        |
|  - "Total violations decreased 15% (120 -> 102) over 30 days"    |
|  - "@frontend-team: biggest improvement (-23)"                    |
|  - "no-explicit-any: biggest regression (+12)"                    |
+------------------------------------------------------------------+
```

### Views

1. **Total** — single line: `total_violations` over time
2. **By Owner** — one line per CODEOWNERS team (from `summary.by_owner`)
3. **By Category** — one line per category (eslint, typescript, etc.)
4. **By Rule** — top 10 rules by latest count, filterable
5. **By Directory** — top-level directories, filterable by depth

### Insights (auto-generated)

Computed by comparing first and last reports:
- Overall trend (up/down, percentage, absolute)
- Biggest improver and biggest regressor per dimension
- Concentration stats ("src/legacy/ accounts for 45% of violations")

## build.rs Strategy

```rust
// build.rs
fn main() {
    // Only rebuild frontend when web/ sources change
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/index.html");
    println!("cargo:rerun-if-changed=web/package.json");

    let web_dir = std::path::Path::new("web");
    if !web_dir.join("node_modules").exists() {
        // Run npm install
        Command::new("npm").args(["install"]).current_dir(web_dir).status()...
    }
    // Run npm run build
    Command::new("npm").args(["run", "build"]).current_dir(web_dir).status()...
}
```

rust-embed points at `web/dist/` which is Vite's output directory.

## Implementation Steps

1. **Add Rust dependencies** to Cargo.toml
2. **Scaffold Vue project** — `web/` with Vite, Vue 3, Tailwind, Chart.js, TypeScript
3. **Create `build.rs`** — auto-build frontend during cargo build
4. **Create `src/trend/loader.rs`** — load and validate JSON reports from paths/directories
5. **Create `src/trend/server.rs`** — axum router with rust-embed serving `web/dist/` and `/api/reports`
6. **Create `src/trend/mod.rs`** — wire loader + server, handle tokio runtime
7. **Update `src/cli.rs`** — add `Trend` subcommand
8. **Update `src/main.rs`** — add `mod trend` and match arm
9. **Build frontend types** — `web/src/types.ts` mirroring Rust Report types
10. **Build composables** — `useTrends.ts` (series extraction, delta computation, insights) and `useReports.ts` (fetch + upload)
11. **Build components** — DimensionTabs, TrendChart, FilterPanel, SummaryTable, InsightsPanel
12. **Wire up App.vue** — compose all components with reactive state
13. **Add `POST /api/reports`** — accept uploaded reports for drag-and-drop
14. **Tests** — Rust loader unit tests, verify cargo build triggers frontend build, manual dashboard verification

## Verification

1. `cargo build` — should auto-build the Vue frontend and compile everything
2. Generate 3+ test fixture reports with different timestamps
3. `cargo run -- trend /tmp/reports/` — should open browser with dashboard
4. Verify all 5 views render correctly with trend lines
5. Verify filter checkboxes toggle chart lines
6. Verify summary table shows first/last/delta
7. `cargo test` — all existing + new tests pass
