# Projector Project Intelligence Suite Design

> Design date: 2026-05-30
> Status: implemented

## Background

Projector already provides full project scanning, snapshots, reports, trends, and export. The current version can answer "how healthy are my projects" but not cross-project questions like "which of my dependencies are shared", "which projects are dead", or "what have I been working on recently".

This extension follows the **B+C+D** direction (cross-project intelligence + deep analysis + discovery & organization), introducing three new commands and a tag system.

## Design Principles

- **Incremental delivery**: each command is independently releasable and non-blocking
- **Reuse the existing architecture**: dependency analysis reuses the analyzer's file-walking logic; tags hook into the existing `list`/`report`/`scan` flows
- **Zero external dependencies**: all dependency file parsing uses the existing `toml` and `serde_json` crates
- **Compatibility**: never break existing commands and behavior

## Scope Overview

| Command | Purpose | Core file changes |
|---------|---------|-------------------|
| `projector deps` | cross-project dependency analysis | new `src/subcmd/deps.rs`; new analyzer dep-parsing functions |
| `projector orphans` | locate orphan projects | new `src/subcmd/orphans.rs`; reuses snapshot data |
| `projector activity` | recent activity summary | new `src/subcmd/activity.rs`; reuses `count_commits` |
| `projector tag` | project tag management | new `src/subcmd/tag.rs`; new `src/tags.rs`; enhance `list.rs`/`report.rs` |
| `projector search` | cross-project search | new `src/subcmd/search.rs` |

## Data Model

### Tag Index (`~/.projector/tags.toml`)

```toml
[tags]
frontend = ["/home/user/projects/web-app", "/home/user/projects/blog"]
archived = ["/home/user/projects/old-tool"]
rust = ["/home/user/projects/projector"]
```

Rust struct:

```rust
struct TagsIndex {
    tags: HashMap<String, Vec<String>>,  // tag_name -> [project_paths]
}
```

### Dependency Entry (computed, not persisted)

```rust
struct DependencyEntry {
    name: String,          // dependency name, e.g. "serde"
    version_req: String,   // version constraint, e.g. "1.0" or "^2.0"
    project_path: String,  // owning project path
    dep_type: String,      // "rust" / "js" / "python" / "go"
    is_dev: bool,          // whether it is a dev dependency
}
```

## Command Details

### 1. `projector deps`

Cross-project dependency aggregation and analysis.

**CLI**:

```
projector deps                           # full dependency report
projector deps <path>                    # dependencies of a specific project
projector deps --shared                  # only cross-project shared deps
projector deps --project <name>          # filter by project
projector deps -f json                   # JSON output
```

**Dependency file parsing scope**:

| Language | File | Parse method | Extracted fields |
|----------|------|--------------|------------------|
| Rust | Cargo.toml | `toml` (existing dep) | keys + version from `[dependencies]`, `[dev-dependencies]` |
| JS/TS | package.json | `serde_json` (existing dep) | `dependencies`, `devDependencies` |
| Go | go.mod | line-by-line `require (...)` blocks | module path + version |
| Python | pyproject.toml | `toml` | `project.dependencies` / `[tool.poetry.dependencies]` |
| Python | requirements.txt | line-by-line | package name + version constraint |

**Terminal output:**

```
  Dependency Report — 24 projects
  ========================================
  Shared dependencies (used by 2+ projects):
    serde          1.0    Rust    used by: projector, my-lib, web-rs
    tokio          1.35   Rust    used by: projector, web-rs
    react          18.2   JS      used by: web-app, admin-panel
    axios          1.6    JS      used by: web-app, admin-panel, api-gateway

  Per-project:
    projector       Rust    12 deps (1 dev)
    web-app         JS      24 deps (8 dev)
    ...
```

**JSON output structure**:

```json
{
  "shared": [{"name": "serde", "version": "1.0", "type": "rust", "projects": ["proj1", "proj2"]}],
  "projects": [{"path": "projector", "total_deps": 12, "dev_deps": 1, "deps": [...]}],
  "total_projects": 24,
  "total_deps": 156,
  "unique_deps": 89,
  "shared_dep_count": 12
}
```

**Edge cases**:
- no scannable projects (no snapshot) → "run `projector scan` first"
- project without dependency files → empty deps array
- failed dependency file parse → skip the project, print a warning
- `--shared` with no shared deps → "no cross-project shared dependencies"
- single-project mode (`projector deps <path>`) does not need a snapshot

### 2. `projector orphans`

Locate "orphan" projects — no remote tracking and no recent local activity.

**CLI**:

```
projector orphans                          # default 90-day threshold
projector orphans --days 180               # custom inactive days
projector orphans -f json                  # JSON output
projector orphans --all                    # show all project states (incl. non-orphans)
```

**Classification**:
- has a `.git` directory
- no `origin` remote tracking (or never pushed)
- latest commit older than `--days` (default 90)
- all conditions met → marked "orphan"

**Output**:

```
  Orphan Projects (no remote + no activity >90d)
  ================================================
    old-experiment     Rust     last commit: 2025-01-15 (501d ago)
    playground         Python   last commit: 2025-03-20 (436d ago)
    scratchpad         JS       last commit: 2025-06-01 (363d ago)

  Total: 3 orphan projects out of 24 scanned
```

**Edge cases**:
- no snapshot → "run `projector scan` first"
- no orphans → "🎉 no orphan projects"
- project without git → not counted; marked "No git" with `--all`

### 3. `projector activity`

Cross-project recent activity summary.

**CLI**:

```
projector activity                         # default: last 7 days
projector activity --days 30               # last 30 days
projector activity -f json                 # JSON output
projector activity --project <path>        # single-project activity details
```

**Data source**: live `git log` per project, reusing `count_commits`; does not depend on snapshots.

**Output**:

```
  Activity — Last 7 days
  ========================================
  Total commits:       24
  Active projects:     5 / 24

  Hottest projects:
    projector          Rust     12 commits  (2 authors)
    web-app            JS        6 commits  (1 author)
    blog               Python    3 commits  (1 author)
    ...

  Idle projects (no activity):
    old-tool           Rust      last commit 2025-11-30 (181d ago)
    playground         Go        last commit 2026-01-15 (135d ago)
```

**Edge cases**:
- git log fails for a project → skip it, print a warning
- no active projects → "no activity detected in the last N days"
- `--project <path>` shows that project's commit history summary

### 4. `projector tag`

Project tag system.

**CLI**:

```
projector tag list [<path>]                # list all tags / a project's tags
projector tag set <path> <tag>             # add a tag
projector tag rm <path> <tag>              # remove a tag
projector tag clear <path>                 # clear all tags
```

**Integration**:

- `projector list --tag <tag>` — show only projects with the tag
- `projector report --filter tag=archived` — report filtering supports the `tag` field
- `projector scan` reads tags automatically (does not affect scan logic; used for reporting)

**Storage**: `~/.projector/tags.toml`, independent of config and snapshots.

**Edge cases**:
- empty tag name or a tag containing spaces → error
- path does not exist or was never scanned → allowed (tags are metadata, path is not validated)
- `tag rm` with a non-existent tag → silent success
- path not present in any tag → `tag list <path>` returns empty

### 5. `projector search`

Cross-project search. Lightweight text search without a full-text engine.

**CLI**:

```
projector search <query>                   # search name/type/path
projector search <query> --tag <tag>       # search within a tag
projector search <query> -f json           # JSON output
```

**Search scope**: based on the latest snapshot, matching:
- project name (last path component)
- full project path
- project type
- tag names

**Output**:

```
  Search results for "web"
  ========================================
    web-app             JS           tag: frontend
    web-rs              Rust         tag: backend
    old-web-demo        JS           tag: archived

  3 results
```

**Edge cases**:
- no snapshot → "run `projector scan` first"
- no results → "no matching projects"
- empty query → error

## File Changes

| File | Change | Description |
|------|--------|-------------|
| `src/subcmd/deps.rs` | add | dependency analysis subcommand |
| `src/subcmd/orphans.rs` | add | orphan project subcommand |
| `src/subcmd/activity.rs` | add | activity summary subcommand |
| `src/subcmd/tag.rs` | add | tag management subcommand |
| `src/subcmd/search.rs` | add | search subcommand |
| `src/tags.rs` | add | tag index IO module |
| analyzer | enhance | dependency parsing functions (`parse_dependencies`, `parse_cargo_deps`, `parse_package_json_deps`, `parse_go_mod_deps`, `parse_pyproject_deps`, `parse_requirements_txt`) |
| `src/command.rs` | enhance | 5 new subcommands and args |
| `src/subcmd/mod.rs` | modify | register new modules |
| `src/lib.rs` | modify | register the `tags` module |
| `src/subcmd/list.rs` | enhance | `--tag` filtering |
| `src/subcmd/report.rs` | enhance | `tag` filter field |

## Config Changes

None. Tags live in the separate file `~/.projector/tags.toml`; `config.toml` is untouched.

## Dependency Changes

None. All parsing uses the existing `toml` and `serde_json` crates.

## Architecture Impact

- The tag system (`tags.rs`) is a standalone module with no dependency on snapshot or config
- Dependency parsing lives in the analyzer alongside helpers like `file_type_distribution`
- `deps` and `activity` can run without snapshots (direct filesystem access); `orphans` and `search` need snapshots
- `tag` integrates into `list`/`report` via `--tag`, minimally invasive

## Release Order

1. **Phase 1: Tag system** — `tag` subcommand + `tags.rs` + `list --tag` + `report --filter tag=`
2. **Phase 2: Dependency analysis** — `deps` subcommand + analyzer dep parsing
3. **Phase 3: Search + activity** — `search` + `activity` subcommands
4. **Phase 4: Orphan detection** — `orphans` subcommand

Each phase is independently releasable; sequential development is recommended (P1's tag system is reused by P2–P4's search/filter).