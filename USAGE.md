# Projector Manual

> Personal project statistics tool — scan git projects, analyze health, track trends

Version: 0.2.0 | Config path: `~/.projector/config.toml` | Snapshot path: `~/.projector/snapshots/`

---

## Install

```bash
cargo install projector
```

## Command Overview

| Command | Purpose |
|---------|---------|
| [`list`](#list) | List projects / non-project directories under a path |
| [`scan`](#scan) | Scan projects and save a JSON snapshot |
| [`report`](#report) | Health dashboard with diff, sorting, filtering |
| [`config`](#config) | View / change config |
| [`activity`](#activity) | Commit activity stats |
| [`deps`](#deps) | Dependency analysis |
| [`orphans`](#orphans) | Find orphan projects (no remote + inactive) |
| [`search`](#search) | Search projects (name, path, type, tags) |
| [`tag`](#tag) | Project tag management |
| [`rank`](#rank) | Rank projects (health, LOC, commits, …) |
| [`brief`](#brief) | Project brief |
| [`size`](#size) | Directory size analysis |
| [`inspect`](#inspect) | Deep analysis of a single project |
| [`stats`](#stats) | Global statistics |
| [`trend`](#trend) | Cross-snapshot trend chart (ASCII) |
| [`snapshot`](#snapshot) | Snapshot management (prune old snapshots, migrate schema) |
| [`export`](#export) | Export HTML dashboard |
| [`completion`](#completion) | Generate shell completion scripts |

---

## list

List git project directories and plain directories under a path.

```bash
projector list [dir] [--tag <tag>]
```

- `dir` — target directory, default `.`
- `--tag <tag>` — only show projects with the given tag

Output: each project's name, detected language type, last modified time, and tags (if any). Dates older than 30 days are printed in red. Projects found below the first level also show their `depth`.

The search depth follows the config key `scan.max_depth` (default 1 = direct children only).

```bash
projector list                         # list projects in the current directory
projector list ~/projects              # list a specific directory
projector list --tag work              # only projects tagged "work"
```

---

## scan

Scan git projects in a directory, generating a snapshot saved to `~/.projector/snapshots/`.

```bash
projector scan [dir]
```

- `dir` — target directory, defaults to config `scan.default_path`

Discovery depth is controlled by `scan.max_depth` (default `1`):

| `max_depth` | Behaviour |
|-------------|-----------|
| `1` | Only direct children of `dir` — the historical behaviour |
| `2`+ | Descend up to N levels to find nested repositories |
| `0` | Unlimited (hard-capped at 32 levels as a safety guard) |

A discovered repository is **not** descended into, so nested repos inside a repo (vendored checkouts, submodules) are not reported as separate projects. `node_modules/` and `target/` are never entered, and symlinks are classified but never descended into, so link cycles cannot hang the walk.

During scan:
- skip hidden directories (starting with `.`)
- detect each project's language type
- analyze git health (branch, dirty state, unpushed commits, last commit time)
- estimate lines of code (LOC)
- compute a health score
- save a JSON snapshot

If a project's health score is below `alert.health_threshold` (default 40), a warning with the deduction reasons is printed at the end.

---

## report

Show the health dashboard based on the latest snapshot.

```bash
projector report [--diff] [-f json|md] [--sort <field>] [--filter <expr>...]
```

**Options:**

| Option | Description |
|--------|-------------|
| `--diff` | Diff between the last two snapshots |
| `-f, --format` | Output format: `json` or `md` (default terminal table) |
| `--sort <field>` | Sort field, prefix `-` for descending |
| `--filter <expr>` | Filter expression, repeatable |

**Sort fields:** `name` `type` `health` `loc` `lines_of_code` `branch` `last_commit`

```bash
projector report -f json              # JSON output
projector report -f md                # Markdown table output
projector report --diff               # diff against previous snapshot
projector report --sort -health       # descending by health
projector report --sort name          # ascending by name
```

**Filter syntax:** `<field>[:<op>]=<value>`

- Operators: `eq` (default), `gte`, `lte`, `gt`, `lt`
- Filterable fields: `name` `type` `health` `loc` `dirty` `branch` `last_commit`

```bash
projector report --filter "type=Rust"                               # only Rust projects
projector report --filter "health:gte=80"                           # health ≥ 80
projector report --filter "dirty=true"                              # only dirty projects
projector report --filter "type=Rust" --filter "health:gte=80"      # multiple conditions (AND)
```

---

## config

View or change the config.

```bash
projector config              # show the current config
projector config set <key> <value>   # change a config entry
```

**Config keys:**

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `scan.default_path` | string | `.` | Default scan directory for `scan` |
| `scan.max_depth` | number | 1 | Levels below the scan root to search for repos (`0` = unlimited) |
| `report.stale_threshold_days` | number | 90 | Days without commits to be considered stale |
| `snapshot.keep_count` | number | 30 | Snapshots kept by `snapshot prune` by default |
| `alert.health_threshold` | number | 40 | Warn when a scanned project's health is below this |

```bash
projector config set scan.default_path ~/projects
projector config set report.stale_threshold_days 60
projector config set scan.max_depth 2      # also find repos one level deeper
```

Config file location: `~/.projector/config.toml`

---

## activity

Commit activity stats per project, based on the project list in the latest snapshot. With `--project`, analyze that directory directly (no snapshot needed).

```bash
projector activity [--days <N>] [--project <path>] [-f json]
```

| Option | Description |
|--------|-------------|
| `--days <N>` | Count commits in the last N days (default 7) |
| `--project <path>` | Analyze only the given project path |
| `-f, --format` | Output format: `json` |

```bash
projector activity                         # all projects, last 7 days
projector activity --days 7                # last week
projector activity --project ~/projects/myapp   # single project
projector activity -f json                 # JSON
```

Output:
- total commits, active projects / total projects
- most active projects (descending by commits)
- idle projects (no commit activity)

---

## deps

Dependency analysis. Supports Rust (Cargo.toml), JavaScript (package.json), Go (go.mod), Python (pyproject.toml/requirements.txt).

```bash
projector deps [path] [--shared] [--project <name>] [-f json]
```

| Option | Description |
|--------|-------------|
| `path` | Project path (optional; scans all projects in the latest snapshot) |
| `--shared` | Show only dependencies shared by 2+ projects |
| `--project <name>` | Filter by project name (fuzzy match) |
| `-f, --format` | Output format: `json` |

```bash
projector deps                                            # overview of all projects
projector deps ~/projects/myapp                           # single project
projector deps --shared                                   # shared (cross-project) deps
projector deps --project myapp                            # only projects named "myapp"
projector deps --shared -f json                           # shared deps as JSON
```

`--shared` output: for each shared dependency, show name, version, type, and the projects using it. When projects pin **different version requirements** for the same dependency, it is flagged with `⚠ conflicting requirements`: for example `serde` with `1.0` in one project and `^2.0` in another. The JSON report carries `conflict`, the full `versions` list, and a `conflicts_count` summary.

---

## orphans

Find orphan projects — projects with no configured remote `origin` and no commits in the last N days.

```bash
projector orphans [--days <N>] [--all] [-f json]
```

| Option | Description |
|--------|-------------|
| `--days <N>` | Inactivity threshold in days (default 90) |
| `--all` | Also show non-orphan and non-git projects |
| `-f, --format` | Output format: `json` |

```bash
projector orphans                         # orphan projects (90-day default)
projector orphans --days 30               # stricter: 30 days
projector orphans --all                   # full view (including non-orphans)
projector orphans -f json                 # JSON
```

---

## rank

Rank all projects using the latest snapshot.

```bash
projector rank [--by <metric>] [--reverse] [--type <filter>] [--top <N>] [--category] [-f json]
```

| Option | Description |
|--------|-------------|
| `--by <metric>` | Metric: `health` (default), `loc`, `activity` (commits in last 30 days), `age` (days since last commit), `commits` (total commits) |
| `--reverse` | Ascending order (default descending) |
| `--type <filter>` | Filter by project type (fuzzy match) |
| `--top <N>` | Show only the top N |
| `--category` | Group by type, show only the best of each type |
| `-f, --format` | Output format: `json` |

```bash
projector rank                          # descending by health
projector rank --by loc --top 5         # top 5 by lines of code
projector rank --by activity            # most active in the last 30 days
projector rank --by age --reverse       # youngest (recently committed)
projector rank --type Rust --top 3      # top 3 Rust projects
projector rank --category               # best project of each language
projector rank --by commits -f json     # JSON
```

---

## brief

Project brief based on the latest snapshot: totals, health distribution, activity.

```bash
projector brief [--days <N>] [-f json]
```

| Option | Description |
|--------|-------------|
| `--days <N>` | Commit activity window in days (default 1) |
| `-f, --format` | Output format: `json` |

Output:
- total projects, average health, health distribution (≥80 / 50–79 / <50)
- total LOC, dirty projects, stale projects
- projects with commits in the last N days (descending by commit count)

```bash
projector brief                         # last 1 day
projector brief --days 7               # last 7 days
projector brief --days 30 -f json       # last 30 days, JSON
```

---

## size

Disk usage analysis for project directories.

```bash
projector size [path] [--top <N>] [--deep] [-f json]
```

| Option | Description |
|--------|-------------|
| `path` | Project path (optional) |
| `--top <N>` | Show only the N largest projects (based on snapshot when no `path`) |
| `--deep` | With `path`, break down into source / deps / git / other |
| `-f, --format` | Output format: `json` |

```bash
projector size                        # all projects from the latest snapshot
projector size --top 5                # 5 largest projects
projector size ~/projects/myapp       # single project
projector size ~/projects/myapp --deep   # deep breakdown
projector size -f json                # JSON
```

`--deep` output categories:
- **Source** — total size of files with common code extensions
- **Dependencies** — `node_modules`, `target` directories
- **Git** — `.git` directory
- **Other** — the rest

---
## search

Search projects in the latest snapshot by name, path, type, and tags.

```bash
projector search <query> [--tag <tag>] [-f json]
```

| Option | Description |
|--------|-------------|
| `query` | Search term (required) |
| `--tag <tag>` | Only projects with the given tag |
| `-f, --format` | Output format: `json` |

```bash
projector search rust                     # name/path/type containing "rust"
projector search myapp                    # project name containing "myapp"
projector search web --tag work           # "web" among projects tagged "work"
projector search rust -f json             # JSON
```

---

## tag

Manage project tags, stored in `~/.projector/tags.toml`.

```bash
projector tag list [path]          # list all tags or a project's tags
projector tag set <path> <tag>     # add a tag to a project
projector tag rm <path> <tag>      # remove a tag
projector tag clear <path>         # clear all tags of a project
```

```bash
projector tag list                          # all tags with usage counts
projector tag list ~/projects/myapp         # tags of one project
projector tag set ~/projects/myapp work     # tag as "work"
projector tag set ~/projects/myapp rust     # tag as "rust"
projector tag rm ~/projects/myapp work      # remove "work"
projector tag clear ~/projects/myapp        # clear all tags
```

Tags are used by `list --tag` and `search --tag` for filtering.

---

## inspect

Deep analysis of a single project directory.

```bash
projector inspect [path] [-f json]
```

- `path` — project path, default `.`
- `-f, --format` — only `json` is supported

Output:
- project type, LOC, git branch, health score
- commit activity (total / last 30 days / last 90 days / last year / authors)
- file type distribution (grouped by language with percentages)
- extra health metrics (stash count, untracked files, commits behind upstream)
- health score deduction reasons

```bash
projector inspect                        # current directory
projector inspect ~/projects/myapp       # specific project
projector inspect -f json                # JSON
```

---

## stats

Global statistics from the latest snapshot.

```bash
projector stats [-f json]
```

- `-f, --format` — `json` (default terminal table)

Output:
- project count, total LOC, average / median / std-dev health
- health distribution: ≥80 (good) / 50-79 (fair) / <50 (poor)
- dirty ratio, stale ratio
- language type distribution
- Top 5 / Bottom 5 projects

---

## trend

ASCII trend chart across snapshots.

```bash
projector trend [path] [--days <N>] [--metric <name>] [-f json]
```

| Option | Description |
|--------|-------------|
| `path` | Project path (optional, defaults to all projects aggregated) |
| `--days <N>` | Only snapshots from the last N days |
| `--metric` | `health` (default) or `loc` |
| `-f, --format` | `json` |

```bash
projector trend                        # average health trend
projector trend myapp                  # single project health trend
projector trend --metric loc           # LOC trend
projector trend --days 90              # last 90 days
projector trend -f json                # JSON
```

---

## snapshot

Manage snapshot files.

```bash
projector snapshot prune [--keep <N>] [--dry-run]
projector snapshot migrate
```

### `snapshot prune`

| Option | Description |
|--------|-------------|
| `--keep <N>` | Keep the newest N snapshots, default from config `snapshot.keep_count` (30) |
| `--dry-run` | Simulate; do not delete anything |

### `snapshot migrate`

Rewrite snapshot files written by an older projector so they carry the current
`schema_version`.

Loading old snapshots **works without migrating** — every snapshot field has a
serde default, so missing fields are backfilled in memory on each read.
Running `migrate` just persists that backfill once, so files stop being patched
on every load. Files that cannot be parsed are reported and left untouched.

```bash
projector snapshot prune               # keep the newest 30
projector snapshot prune --keep 10     # keep 10
projector snapshot prune --dry-run     # preview what would be deleted
projector snapshot migrate             # bring old snapshots up to the current schema
```

---

## export

Export an HTML dashboard.

```bash
projector export html [-o <output>]
```

| Option | Description |
|--------|-------------|
| `-o, --output` | Output HTML file path (default stdout) |

```bash
projector export html                                   # print HTML to stdout
projector export html -o dashboard.html                 # write to a file
```

---

## completion

Generate shell completion scripts.

```bash
projector completion <bash|zsh|fish>
```

```bash
projector completion bash > /etc/bash_completion.d/projector
projector completion zsh > /usr/local/share/zsh/site-functions/_projector
projector completion fish > ~/.config/fish/completions/projector.fish
```

---

## Health Score

Starts at **100** and deducts for risk factors:

| Deduction | Condition |
|-----------|-----------|
| -15 | no commits for `stale_threshold_days` (default 90) |
| -10 | dirty working tree |
| -5 × N | unpushed commits, 5 points per 5 commits |
| -10 | files last modified over 60 days ago |
| -5 | fewer than 100 lines of code (possibly abandoned scaffold) |

Clamped to **0–100**. Terminal output is color-coded:
- **≥80** green — healthy
- **50–79** yellow — fair
- **<50** red — poor

---

## Snapshot Storage

- Snapshot files: `~/.projector/snapshots/{YYYYMMDD_HHMMSS}.json`
- Each `scan` produces a new snapshot
- `report --diff` compares the last two snapshots
- `trend` uses all historical snapshots
- `snapshot prune` removes old snapshots
- Each file records a `schema_version`; older files stay readable because new
  fields are added with serde defaults and backfilled by `snapshot migrate`

Each project entry stores: `path`, `project_type`, `git_branch`, `is_dirty`,
`unpushed_commits`, `last_commit_date`, `last_modified_date`, `lines_of_code`,
`health_score`, and `depth` (how far below the scanned root the repo was found).

---

## Tag Storage

- Tag file: `~/.projector/tags.toml`
- TOML mapping of project paths to tag lists
- Used for `list --tag` and `search --tag` filtering
- Managed independently of snapshots; never lost by `snapshot prune`

---

## Config Reference

A complete `~/.projector/config.toml`:

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

## Recommended Workflow

```bash
# 1. Set the scan directory
projector config set scan.default_path ~/projects

# 2. First scan
projector scan

# 3. View the dashboard
projector report

# 4. Tag projects
projector tag set ~/projects/work-project work
projector tag set ~/projects/hobby-project hobby

# 5. Filter by tag
projector list --tag work

# 6. Deep-analyze a problematic project
projector inspect ~/projects/some-project

# 7. Check activity trends
projector activity --days 30

# 8. Find orphan projects
projector orphans

# 9. Find cross-project shared dependencies
projector deps --shared

# 10. Scan periodically (e.g. via cron)
projector scan && projector report --diff

# 11. Export a dashboard to share
projector export html -o dashboard.html

# 12. Prune old snapshots
projector snapshot prune --keep 20
```
