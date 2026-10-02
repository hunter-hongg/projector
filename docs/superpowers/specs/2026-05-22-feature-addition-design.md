# Projector Feature Extension Design

> Design date: 2026-05-22
> Status: implemented

## Background

Projector is a personal project statistics Rust CLI. The current version (v0.0.1) provides project discovery, scan analysis, snapshot storage, and report display. This extension follows the "A+C" direction (deep insights + workflow integration), delivered in four phases.

## Design Principles

- **YAGNI**: only build what is clearly needed; no over-engineering
- **Consistent with the existing architecture**: keep the analyzer → snapshot → subcmd layering
- **Incremental delivery**: each phase can be released independently
- **Compatibility**: never break existing commands and behavior

## Scope Overview

| Phase | Features | Core file changes |
|-------|---------|-------------------|
| P1 | `inspect`, `stats` | new `src/subcmd/inspect.rs`, `src/subcmd/stats.rs`; enhance analyzer |
| P2 | `trend`, `report --sort/--filter` | new `src/subcmd/trend.rs`; enhance `report.rs`, `snapshot.rs` |
| P3 | `completion`, `export html` | new `src/subcmd/completion.rs`, `src/subcmd/export.rs`; new dependency `clap_complete` |
| P4 | `snapshot prune`, health alert | new `src/subcmd/snapshot.rs`; enhance `scan.rs`, `config.rs` |

---

## Phase 1: Deep Insights

### 1.1 `projector inspect <path>`

Deep analysis of a single project, whether or not it has been scanned before.

**CLI**:

```
projector inspect [path]             # default to the current directory
projector inspect ~/projects/foo     # specific path
projector inspect -f json <path>     # JSON output
```

**Data dimensions**:

| Dimension | Implementation |
|-----------|----------------|
| Base info | reuse `analyze_project` for path, type, branch, health score |
| Version activity | walk commit history with `git2`; count totals, N-day frequency, authors |
| Code composition | new file-type grouping stats (file counts and LOC aggregated by extension) |
| Repo health | check untracked files, stashes, and whether the branch is behind upstream |

**Edge cases**:
- Path is not a git repo → degraded output (file-type stats only, no git metrics), with a "not a git repo, showing file info only" notice
- Path was never scanned → on-the-fly analysis, with an "on-the-fly analysis (no snapshot found)" notice
- Path does not exist → explicit error message
- `-f json` → structured JSON output for script consumption

**File changes**:
- new `src/subcmd/inspect.rs`
- enhance the analyzer:
  - `count_commits()` — commit counts over different time windows
  - `file_type_distribution()` — file-type grouping stats
  - `git_extra_health()` — stash count, untracked files, behind-upstream amount

**Test requirements**:
- degraded output for non-git directories
- commit frequency boundaries (empty repo, single commit)
- JSON serialization correctness
- file-type distribution correctness

---

### 1.2 `projector stats`

Global aggregate statistics from the latest snapshot.

**CLI**:

```
projector stats                   # terminal table output
projector stats -f json           # JSON output
```

**Statistics**:

| Statistic | Calculation |
|-----------|-------------|
| project count | `snapshot.projects.len()` |
| type distribution | grouped counts by `project_type`, with percentages |
| average health | arithmetic mean of `health_score` |
| median health | median of sorted `health_score` |
| standard deviation | population std-dev of health scores |
| health buckets | three counts: ≥80 / 50-79 / <50 |
| Top 5 / Bottom 5 | first/last after sorting by health score |
| total LOC | sum of `lines_of_code` |
| dirty ratio | share of projects with `is_dirty=true` |
| stale ratio | share of projects older than `stale_threshold_days` |

**Edge cases**:
- no snapshot → "no snapshot found, run `projector scan` first"
- zero projects → all statistics are zero, no error
- single project → std-dev is 0

**File changes**:
- new `src/subcmd/stats.rs`
- new analyzer helper `compute_stats(snapshot)`

**Test requirements**:
- empty-snapshot statistics
- single-project statistics (std-dev = 0)
- multi-project numeric correctness
- JSON / terminal format output

---

## Phase 2: Trend Analysis

### 2.1 `projector trend`

Show the health/LOC trend across snapshots over time.

**CLI**:

```
projector trend                           # average health trend of all projects
projector trend <path>                    # single-project trend
projector trend --days 90                 # limit the time range
projector trend --metric loc              # switch to LOC trend
projector trend -f json                   # JSON output
```

**Implementation**:
- scan `~/.projector/snapshots/`, load all snapshots sorted by file name
- "all projects" mode: average health score (or total LOC) per snapshot
- "single project" mode: match the project path in each snapshot and extract its health/LOC series
- ASCII line chart: auto-scaled Y axis
- Y-axis labels left-aligned; X axis shows snapshot dates (density auto-selected)

**Constraints**:
- at least 2 snapshots required to plot (otherwise a notice)
- fewer than 3 snapshots → no trend line, data points only
- if a single project is missing from a snapshot → skip that point (no error)
- `--days` filters the snapshot time range and composes with the single-project mode

**File changes**:
- new `src/subcmd/trend.rs`
- `src/snapshot.rs`: `load_all()` to load all snapshots
- analyzer: ASCII chart drawing function

**Test requirements**:
- boundary behavior with 0/1/2 snapshots
- correct trend series extraction
- skip logic for missing single-project snapshots
- JSON numeric series correctness

---

### 2.2 `report --sort` & `report --filter`

Enhance `report` with sorting and filtering.

**CLI**:

```
projector report --sort health              # ascending
projector report --sort -health             # descending
projector report --sort loc
projector report --sort name

projector report --filter type=Rust
projector report --filter health:gte=80
projector report --filter health:lte=50
projector report --filter dirty=true
projector report --filter type=Python --sort -health  # combined
```

**Sort fields**: `name`, `type`, `health`, `loc`, `branch`, `last_commit`

> Descending: the `--sort` value parser detects a `-` prefix (e.g. `-health`), treats the rest as the field name and reverses sort direction. clap does not misparse `-health` as a flag because `--sort` consumes a value token.

**Filter syntax**: `<field>[:<op>]=<value>`, the `:<op>` part is optional (default `eq`):
- `type=Rust` is equivalent to `type:eq=Rust`
- `health:gte=80` — supports `eq`, `gte`, `lte`, `gt`, `lt`

**Implementation**:
- sorting: `.sort_by()` on `latest.projects`
- filtering: `.filter()` on `latest.projects`
- filter first, then sort
- fully compatible with `-f json` / `-f md`
- invalid field name → "invalid sort/filter field", listing available fields
- invalid operator → "invalid operator", listing available operators

**File changes**:
- enhance `src/subcmd/report.rs`: parse `--sort` and `--filter`
- helper functions for sorting/filtering
- update `src/command.rs`: new CLI arguments

**Test requirements**:
- correctness of each sort field
- correctness of each filter operator (eq/gte/lte/gt/lt)
- multi-condition AND filter
- sort + filter combination
- error messages for invalid fields/operators

---

## Phase 3: Workflow Integration

### 3.1 `projector completion <shell>`

Generate shell completion scripts.

**CLI**:

```
projector completion bash       # bash completions
projector completion zsh        # zsh completions
projector completion fish       # fish completions
```

**Implementation**:
- new dependency `clap_complete`
- generated automatically via `clap::Command::complete()`
- zero handwritten completion logic

**Constraints**:
- unsupported shell → "unsupported shell: {shell}; supported: bash/zsh/fish"
- output to stdout only

**File changes**:
- new `src/subcmd/completion.rs`
- `Cargo.toml`: add `clap_complete`
- `src/command.rs`: add the new subcommand

**Test requirements**:
- completion script generation per shell (non-empty output, contains `_projector` etc.)
- error for invalid shell type

---

### 3.2 `projector export html`

Generate a self-contained HTML dashboard.

**CLI**:

```
projector export html                        # stdout
projector export html -o dashboard.html      # write to file
```

**HTML contents**:
- overview cards: project count, average health, total LOC, dirty ratio, stale ratio
- health distribution bar chart: three buckets, pure-CSS bars (no JS chart library)
- project table: name, type, branch, status, health, sorted by health
- type distribution: bar chart of project counts by type
- Top 5 / Bottom 5 rankings

**Quality requirements**:
- single file, all CSS/JS inline
- responsive, mobile friendly
- dark/light auto-adaptation (`prefers-color-scheme`)
- no external resources (no CDN, no font libraries)

**File changes**:
- new `src/subcmd/export.rs`
- new `src/export_template.rs` (HTML template as a Rust constant)
- enhance `src/snapshot.rs`: data structures for HTML rendering

**Edge cases**:
- no snapshot → simplified page with a "run scan first" notice
- zero projects → empty-state page
- `-o` path with missing parent directory → create it automatically

**Test requirements**:
- HTML output contains key DOM elements (#app, .stats-card, .project-table, …)
- empty-state page when no snapshot
- `-o` file write correctness
- color correctness across health distributions

---

## Phase 4: Operations

### 4.1 `projector snapshot prune`

Prune old snapshots, keeping only the newest N.

**CLI**:

```
projector snapshot prune --keep 10           # keep 10
projector snapshot prune --keep 30           # default
projector snapshot prune --dry-run           # preview mode
```

**Config extension**: `config.toml` gains:

```toml
[snapshot]
keep_count = 30
```

**Implementation**:
- sort by file-name timestamp (`YYYYMMDD_HHMMSS`), keep the newest `--keep`
- nothing deleted when `--keep` ≥ the snapshot count
- `--keep` minimum is 1; lower values error out
- print deleted file names
- `--dry-run` lists would-be deletions without deleting

**Config command**: `projector config set snapshot.keep_count 50`

**File changes**:
- new `src/subcmd/snapshot.rs`
- enhance `src/config.rs` with the `snapshot` section
- enhance `src/snapshot.rs` with the `prune()` method

**Test requirements**:
- no-op when `--keep` ≥ count
- `--keep 0` errors
- dry-run deletes nothing
- correct remaining snapshot count after pruning
- read/write of new config keys

---

### 4.2 Health threshold alert

Automatically check and print alerts after `scan` finishes.

**Config extension**: `config.toml` gains:

```toml
[alert]
health_threshold = 40
```

**Behavior**:
- only triggered at the end of `scan`; does not affect the exit code
- projects below the threshold listed ascending by health score
- each entry shows the main deductions (from `compute_health_score`)
- threshold 0 disables the alert

**Output example**:

```
Scanned 24 projects, snapshot saved.

⚠  These projects have a health score below 40:
  - old-project     15/100  stale(412d), dirty
  - archived-lib    22/100  stale(2y), loc<100
```

**File changes**:
- enhance `src/subcmd/scan.rs`: check and print alerts after scanning
- enhance `src/config.rs` with the `alert` section

**Test requirements**:
- no alert when threshold is 0
- correct output when projects fall below the threshold
- no alert output when nothing is below the threshold
- read/write of the new config key

---

## Dependency Changes

| Change | Crate | Purpose | Phase |
|--------|-------|---------|-------|
| add | `clap_complete` | auto shell completions | P3 |
| add | none (reuse `git2`) | deep git statistics | P1 |

No other external dependencies. The `export html` template is inlined in Rust source (constant string).

## Config Changes

```toml
[scan]
default_path = "."
max_depth = 1

[report]
stale_threshold_days = 90

[snapshot]          # ← new
keep_count = 30     # ← new

[alert]             # ← new
health_threshold = 40  # ← new
```

## Architecture Impact

- No major architectural changes. All new features are added as new `subcmd` modules
- New analyzer helpers, existing function signatures untouched
- `snapshot.rs` gains `load_all()` and `prune()`
- `config.rs` gains new sections, consistent with the existing pattern

## Release Order

1. **Phase 1** → `projector inspect` + `projector stats`
2. **Phase 2** → `projector trend` + `report --sort/--filter`
3. **Phase 3** → `projector completion` + `projector export html`
4. **Phase 4** → `projector snapshot prune` + alert

Each phase is independently releasable and non-blocking, but sequential development is recommended (P2 depends on P1's snapshot foundation; P4 is fairly independent and can be done earlier).
