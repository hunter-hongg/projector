# 🔦 Projector

> Personal project statistics tool

[![Crates.io](https://img.shields.io/crates/v/projector.svg)](https://crates.io/crates/projector)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## Install

```bash
cargo install projector
```

## Usage

```bash
# List projects in a directory
projector list [dir]

# Scan projects and save a snapshot
projector scan [dir]

# Show the health dashboard
projector report

# Show the diff against the previous snapshot
projector report --diff

# Output JSON / Markdown format
projector report -f json
projector report -f md

# Check for newer upstream dependency versions
projector deps --outdated
projector deps --shared --outdated -f json

# Export a Markdown (or HTML) dashboard
projector export markdown
projector export markdown -o dash.md
projector export html -o dash.html

# Daily brief as Markdown (paste into notes)
projector brief -f md

# Show the current config
projector config

# Change config
projector config set scan.default_path ~/projects
projector config set report.stale_threshold_days 60
projector config set scan.max_depth 2
```

## Commands

Full manual: [USAGE.md](USAGE.md).

| Command | Description |
|---------|-------------|
| `list [dir] [--tag]` | List projects in a directory |
| `scan [dir]` | Scan projects and save a snapshot to `~/.projector/snapshots/` |
| `report [--diff] [-f json\|md]` | Health dashboard with sorting, filtering, diff |
| `activity [--days] [--project]` | Commit activity stats per project |
| `brief [--days] [-f json\|md]` | Daily brief: totals, health distribution, active projects |
| `deps [path] [--shared] [--outdated]` | Dependency analysis (Cargo / npm / go / Python); `--outdated` checks upstream versions |
| `orphans [--days] [--all]` | Find orphan projects (no remote + inactive) |
| `rank [--by] [--top]` | Rank by health / LOC / activity / age / commits |
| `search <query> [--tag]` | Search projects (name, path, type, tags) |
| `size [path] [--top] [--deep]` | Analyze directory sizes |
| `config [set <key> <value>]` | View / change config |
| `inspect [path]` | Deep analysis of a single project |
| `stats` | Global statistics |
| `trend [path] [--metric]` | Cross-snapshot trend chart (ASCII) |
| `completion <shell>` | Generate shell completion scripts |
| `export html\|markdown [-o]` | Export dashboard as HTML or Markdown |
| `snapshot prune\|migrate` | Snapshot management (prune old snapshots, migrate schema) |
| `tag list\|set\|rm\|clear` | Project tag management |

## Config

`~/.projector/config.toml`

```toml
[scan]
default_path = "."
max_depth = 1

[report]
stale_threshold_days = 90

[snapshot]
keep_count = 30

[alert]
health_threshold = 40
```

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `scan.default_path` | string | `.` | Default scan directory for `scan` |
| `scan.max_depth` | number | 1 | Levels below the scan root to search (`0` = unlimited) |
| `report.stale_threshold_days` | number | 90 | Days without commits to be considered stale |
| `snapshot.keep_count` | number | 30 | Snapshots kept by `snapshot prune` by default |
| `alert.health_threshold` | number | 40 | Warn when a scanned project's health is below this |

## Health Score

Starts at 100 and deducts for risk factors:

- **-15** — no commits for `stale_threshold_days` (default 90)
- **-10** — dirty working tree
- **-5 × N** — unpushed commits (5 points per 5 commits)
- **-10** — files last modified over 60 days ago
- **-5** — fewer than 100 lines of code (possibly abandoned scaffold)

Clamped to 0–100. Terminal output is color-coded: ≥80 green, 50–79 yellow, <50 red.

## Storage Paths

| Item | Path |
|------|------|
| Config | `~/.projector/config.toml` |
| Snapshots | `~/.projector/snapshots/{YYYYMMDD_HHMMSS}.json` (versioned by `schema_version`) |
| Tags | `~/.projector/tags.toml` |

## Development

```bash
cargo build        # dev build
cargo test         # run tests (106 unit tests)
cargo build --release
```

Rust edition 2024 (MSRV ≥ 1.85). Dependencies: `anyhow`, `chrono`, `clap`, `clap_complete`, `git2`, `serde`, `serde_json`, `toml`.

## License

MIT © hunter-hongg
