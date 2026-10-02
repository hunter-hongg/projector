# Project Log (Huzhou, 26 Qingming)

## 2026-04-04

- Initialized the project

## 2026-04-05

- Added **README**
- Added **LICENSE**
- Added `clap` for CLI parsing

## 2026-04-07

- Completed the basic `list` command

## 2026-04-18

- Completed the usable `list` command
- Released **0.0.1**

## 2026-05-03

- Basic formatting

## 2026-05-18

- Added `scan`, `report`, `config` subcommands with project analysis
- Added the project design spec and AI agent config

## 2026-05-19

- Refactored the code structure, removed duplication, added test coverage

## 2026-05-20

- Removed duplicated project detection / directory walking code; introduced the `DiffField` enum

## 2026-05-22

- Added the feature-extension design doc (deep insights + workflow integration)

## 2026-05-30

- Added `inspect`, `stats`, `trend`, `completion`, `export`, `snapshot` subcommands
- Added the tag system and `list`, `search`, `orphans`, `deps`, `activity`, `brief` subcommands
- Added the project-intelligence-suite design doc

## 2026-06-08

- Added `calc_dir_size()` and `human_size()` utilities
- Added the `size` subcommand
- Added the `brief` subcommand
- Added the `rank` subcommand
- Updated `USAGE.md`

## 2026-10-01

- Updated the command docs in `AGENTS.md`, `README.md`, `USAGE.md`
- Deleted `CLAUDE.md` (identical to `AGENTS.md`)
- Deleted the `html/` artifacts (unreferenced by any code)
- Added CI: `cargo fmt --check` + `clippy` + `test`
- Ran `cargo fmt` and fixed clippy warnings
- Split `analyzer.rs` into focused modules (detect / git / metrics / health / statistics / chart / dependencies)
- Unified the project language to English (docs, CLI text, comments)

## 2026-10-02

- Added `schema_version` to snapshots with serde defaults, so adding a snapshot field no longer breaks older files on disk
- Added `projector snapshot migrate` to persist the backfill for legacy snapshots
- Implemented the documented-but-missing `scan.max_depth` config key
- `scan`/`list` now discover nested repositories; each project records its `depth`
