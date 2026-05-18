# Project Health Dashboard

> Extending Projector with scan/report/config commands to provide a project health dashboard.

## Architecture

```
scan command                    report command
  │                                 │
  ▼                                 ▼
Scanner ──> SnapshotStore ──> Reporter
               │
               ▼
         ~/.projector/snapshots/*.json
```

- `scan` collects snapshots, `report` reads them
- SnapshotStore handles serialization and diffing
- Config lives at `~/.projector/config.toml`

## Components

### Scanner (`src/subcmd/scan.rs`)
- Walks directory entries (reuses project detection from `list.rs`)
- For each dir with `.git`: runs git analysis via `git2`
- Detects project type (same logic as `list.rs`: Cargo.toml → Rust, package.json → JS/TS, etc.)
- Estimates lines of code (basic file counting, not full tokei integration)

### Reporter (`src/subcmd/report.rs`)
- Reads latest snapshot from SnapshotStore
- Formats as terminal table with color-coded health scores
- Supports `--diff` (compare last two snapshots)
- Supports `-f json` / `-f markdown`

### Config (`src/subcmd/config.rs`)
- Read/write `~/.projector/config.toml`
- Keys: `scan.default_path`, `scan.max_depth`, `report.stale_threshold_days`

### SnapshotStore (`src/snapshot.rs`)
- New module in `src/`
- Save: serialize `Vec<ProjectSnapshot>` to JSON file with timestamp
- Load: read latest or specific snapshot
- Diff: compare two snapshots and return changes

### ProjectAnalyzer (`src/analyzer.rs`)
- New module in `src/`, merges and extends project detection from `list.rs`
- `detect_type(dir) -> ProjectType` — based on config files
- `git_health(dir) -> GitHealth` — branch, dirty, unpushed commits, last commit date
- `estimate_loc(dir) -> u32` — rough line count (`.rs`, `.js`, `.go`, etc.)

## Data Model

```rust
struct ProjectSnapshot {
    path: String,
    project_type: String,
    git_branch: String,
    is_dirty: bool,
    unpushed_commits: u32,
    last_commit_date: NaiveDateTime,
    last_modified_date: NaiveDateTime,
    lines_of_code: u32,
    health_score: u8,  // 0-100
}

struct ScanSnapshot {
    timestamp: NaiveDateTime,
    scanned_path: String,
    projects: Vec<ProjectSnapshot>,
}
```

## Health Score Formula

- Base 100, subtract for each risk factor:
  - `-15` if no commits in 90+ days (stale)
  - `-10` if dirty working tree
  - `-5`  if unpushed commits exist (per 5 commits)
  - `-10` if last modified 60+ days ago
  - `-5`  if lines_of_code < 100 (may be abandoned scaffold)
  - Clamp to 0-100 range

## CLI

```
projector scan [dir]            # scan and save snapshot
projector scan [dir] -f         # force rescan, overwrite latest
projector report                # show health dashboard (latest snapshot)
projector report --diff         # diff against previous snapshot
projector report -f json|md     # output format
projector config                # show config
projector config set <k> <v>    # set config key
```

## Report Output

Terminal table with color bands:
- ≥80 green (healthy)
- 50-79 yellow (needs attention)
- <50 red (at risk)

Columns: Project, Type, Branch, Status (clean/dirty/stale), Last Commit, Health.

## Dependencies to Add

- `git2` — git analysis (branch, dirty, unpushed commits)
- `serde` + `serde_json` — snapshot serialization
- `toml` — config file parsing

## Implementation Order

1. `Config` — TOML config read/write, minimal surface
2. `SnapshotStore` — save/load snapshots, diff logic
3. `ProjectAnalyzer` — type detection (move from list.rs), git analysis, LOC estimation
4. `scan` command — wire up Scanner
5. `report` command — wire up Reporter with terminal table
6. `config` command — wire up Config subcommand
7. Polish: JSON/MD output format, --diff view, color improvements

## Non-goals

- No interactive/TUI mode
- No daemon or file watcher
- No web dashboard
- No per-project deep analysis beyond git + type + LOC
