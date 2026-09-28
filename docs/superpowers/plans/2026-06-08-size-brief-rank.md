# Size / Brief / Rank Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three new subcommands — `projector size` (disk usage), `projector brief` (daily digest), `projector rank` (leaderboard).

**Architecture:** Each subcommand gets its own file under `src/subcmd/`, registered in `command.rs` + `main.rs` dispatch. Shared logic (directory size calculation, human-readable formatting) added to `src/analyzer.rs` since filesystem walk utilities already live there.

**Tech Stack:** Rust, clap derive, existing `analyzer.rs` walk infrastructure, existing snapshot/config code.

---

## File Structure

```
src/
├── analyzer.rs              # + calc_dir_size(), human_size()
├── command.rs               # + Size, Brief, Rank enum variants
├── main.rs                  # + dispatch arms
├── subcmd/
│   ├── mod.rs               # + pub mod size/brief/rank
│   ├── size.rs              # NEW — projector size impl
│   ├── brief.rs             # NEW — projector brief impl
│   └── rank.rs              # NEW — projector rank impl
```

---

### Task 1: Add `calc_dir_size()` and `human_size()` to analyzer.rs

**Files:**
- Modify: `src/analyzer.rs` (after `estimate_loc`, around line 320)
- Test: inline in `src/analyzer.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Write failing tests**

```rust
// Append to existing #[cfg(test)] mod tests at end of src/analyzer.rs

#[test]
fn test_human_size_bytes() {
    assert_eq!(human_size(0), "0 B");
    assert_eq!(human_size(500), "500 B");
}

#[test]
fn test_human_size_kb() {
    assert_eq!(human_size(1_024), "1.0 KB");
    assert_eq!(human_size(12_345), "12.1 KB");
}

#[test]
fn test_human_size_mb() {
    assert_eq!(human_size(1_048_576), "1.0 MB");
    assert_eq!(human_size(3_500_000), "3.3 MB");
}

#[test]
fn test_human_size_gb() {
    assert_eq!(human_size(1_073_741_824), "1.0 GB");
}

#[test]
fn test_calc_dir_size_empty() {
    let dir = std::env::temp_dir().join("projector_test_size_empty");
    let _ = std::fs::create_dir_all(&dir);
    let size = calc_dir_size(&dir, false);
    assert_eq!(size, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_calc_dir_size_with_files() {
    let dir = std::env::temp_dir().join("projector_test_size_files");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(dir.join("a.txt"), "hello").unwrap();
    std::fs::write(dir.join("b.txt"), "world!").unwrap();
    let size = calc_dir_size(&dir, false);
    assert!(size > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_calc_dir_size_deep_skips_hidden() {
    let dir = std::env::temp_dir().join("projector_test_size_deep");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::create_dir_all(dir.join(".hidden"));
    std::fs::write(dir.join(".hidden").join("big.bin"), &vec![0u8; 10_000]).unwrap();
    let size_skip_hidden = calc_dir_size(&dir, true);
    let size_include_hidden = calc_dir_size(&dir, false);
    assert!(size_include_hidden > size_skip_hidden);
    assert!(size_skip_hidden == 0);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo cooldown test test_human_size_bytes test_human_size_kb test_human_size_mb test_human_size_gb test_calc_dir_size_empty test_calc_dir_size_with_files test_calc_dir_size_deep_skips_hidden 2>&1 | head -30`
Expected: each of the 7 tests shows FAIL (function not found)

- [ ] **Step 3: Add `human_size()` function before line 304 (before `COUNTABLE_EXTENSIONS`)**

```rust
pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let bytes_f = bytes as f64;
    let unit_idx = (bytes_f.log10() / 3.0).floor() as usize;
    let unit_idx = unit_idx.min(UNITS.len() - 1);
    let value = bytes_f / (1024u64.pow(unit_idx as u32) as f64);
    if unit_idx == 0 {
        format!("{} {}", value as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", value, UNITS[unit_idx])
    }
}
```

- [ ] **Step 4: Add `calc_dir_size()` function after `human_size()`**

```rust
pub fn calc_dir_size(dir: &Path, skip_hidden: bool) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if skip_hidden && name.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Ok(meta) = fs::metadata(&path) {
                        total += meta.len();
                    }
                }
            }
        }
    }
    total
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo cooldown test test_human_size_bytes test_human_size_kb test_human_size_mb test_human_size_gb test_calc_dir_size_empty test_calc_dir_size_with_files test_calc_dir_size_deep_skips_hidden 2>&1`
Expected: all 7 pass, overall `ok`

- [ ] **Step 6: Run full existing test suite**

Run: `cargo cooldown test 2>&1`
Expected: all existing tests still pass

- [ ] **Step 7: Commit**

```bash
git add src/analyzer.rs
git commit -m "feat: add calc_dir_size() and human_size() utilities"
```

---

### Task 2: Create `projector size` subcommand

**Files:**
- Create: `src/subcmd/size.rs`
- Modify: `src/subcmd/mod.rs` (add `pub mod size;`)
- Modify: `src/command.rs` (add `Size` variant)
- Modify: `src/main.rs` (add dispatch arm)
- Test: inline `#[cfg(test)]` in `src/subcmd/size.rs`

- [ ] **Step 1: Write failing test in new file `src/subcmd/size.rs`**

```rust
use anyhow::Result;

use crate::analyzer;

pub fn subcmd_size(
    path: Option<String>,
    top: Option<usize>,
    deep: bool,
    format: Option<String>,
) -> Result<()> {
    // stub — returns Ok for test
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcmd_size_invalid_format() {
        let result = subcmd_size(None, None, false, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo cooldown test -p projector subcmd_size_invalid_format 2>&1 | head -20`
Expected: FAIL — will compile but test failure

Wait — the stub returns Ok, so the test will fail. That's correct TDD: test demands error for invalid format but stub returns Ok.

- [ ] **Step 3: Implement `subcmd_size` in `src/subcmd/size.rs`**

```rust
use std::path::Path;
use anyhow::Result;

use crate::analyzer;
use crate::color;
use crate::snapshot::SnapshotStore;

pub fn subcmd_size(
    path: Option<String>,
    top: Option<usize>,
    deep: bool,
    format: Option<String>,
) -> Result<()> {
    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    if let Some(p) = path {
        let dir = Path::new(&p);
        if !dir.exists() {
            anyhow::bail!("Path not found: {}", p);
        }
        if deep {
            print_deep_breakdown(dir, &fmt)?;
        } else {
            let size = analyzer::calc_dir_size(dir, true);
            if fmt == "json" {
                let json = serde_json::json!({
                    "path": p,
                    "size": size,
                    "size_human": analyzer::human_size(size),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("  {}: {}", color::cyan(&p), analyzer::human_size(size));
            }
        }
        return Ok(());
    }

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            println!("{}", color::error("No snapshots found. Run `projector scan` first."));
            return Ok(());
        }
    };

    let mut entries: Vec<(String, u64)> = Vec::new();
    for proj in &latest.projects {
        let dir = Path::new(&proj.path);
        if dir.exists() {
            let size = analyzer::calc_dir_size(dir, true);
            entries.push((proj.path.clone(), size));
        }
    }

    entries.sort_by_key(|(_, s)| std::cmp::Reverse(*s));

    if let Some(n) = top {
        entries.truncate(n);
    }

    if fmt == "json" {
        let json: Vec<serde_json::Value> = entries.iter().map(|(path, size)| {
            serde_json::json!({
                "path": path,
                "size": size,
                "size_human": analyzer::human_size(*size),
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!();
        println!("  {}", color::info("Project disk usage"));
        println!();
        let max_name = entries.iter().map(|(p, _)| p.split('/').next_back().unwrap_or(p).len()).max().unwrap_or(20).min(40);
        for (path, size) in &entries {
            let name = path.split('/').next_back().unwrap_or(path);
            println!("  {:max_name$}  {}", color::cyan(name), analyzer::human_size(*size), max_name = max_name);
        }
        println!();
    }

    Ok(())
}

fn print_deep_breakdown(dir: &Path, fmt: &str) -> Result<()> {
    let total = analyzer::calc_dir_size(dir, false);
    let source = dir_size_by_extensions(dir, &["rs", "js", "ts", "jsx", "tsx", "go", "py", "java", "kt", "kts", "c", "h", "cpp", "hpp", "cc", "cxx", "ml", "mli", "dart", "toml", "json", "yaml", "yml", "md", "css", "html", "sh", "bash", "zsh", "fish"]);
    let deps = dir_size_by_name(dir, &["node_modules", "target"]);
    let git = dir_size_by_name(dir, &[".git"]);
    let other = total.saturating_sub(source + deps + git);

    if fmt == "json" {
        let json = serde_json::json!({
            "path": dir.to_string_lossy(),
            "total": total,
            "total_human": analyzer::human_size(total),
            "breakdown": {
                "source": source,
                "deps": deps,
                "git": git,
                "other": other,
            }
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("  Deep breakdown for {}", color::cyan(&dir.to_string_lossy()));
        println!("    Total: {}", analyzer::human_size(total));
        println!("    Source code:  {}", analyzer::human_size(source));
        println!("    Dependencies: {}", analyzer::human_size(deps));
        println!("    Git objects:  {}", analyzer::human_size(git));
        println!("    Other:        {}", analyzer::human_size(other));
    }

    Ok(())
}

fn dir_size_by_extensions(dir: &Path, exts: &[&str]) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') && path.is_dir() {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if exts.contains(&ext) {
                            if let Ok(meta) = std::fs::metadata(&path) {
                                total += meta.len();
                            }
                        }
                    }
                }
            }
        }
    }
    total
}

fn dir_size_by_name(dir: &Path, names: &[&str]) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if names.contains(&name) && path.is_dir() {
                total += analyzer::calc_dir_size(&path, false);
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcmd_size_invalid_format() {
        let result = subcmd_size(None, None, false, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }

    #[test]
    fn test_dir_size_by_extensions_empty() {
        let dir = std::env::temp_dir().join("projector_test_ext_empty");
        let _ = std::fs::create_dir_all(&dir);
        let size = dir_size_by_extensions(&dir, &["rs"]);
        assert_eq!(size, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dir_size_by_name_found() {
        let dir = std::env::temp_dir().join("projector_test_name");
        let _ = std::fs::create_dir_all(dir.join("node_modules"));
        std::fs::write(dir.join("node_modules").join("pkg.js"), "abc").unwrap();
        let size = dir_size_by_name(&dir, &["node_modules"]);
        assert!(size > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dir_size_by_name_not_found() {
        let dir = std::env::temp_dir().join("projector_test_name_miss");
        let _ = std::fs::create_dir_all(&dir);
        let size = dir_size_by_name(&dir, &["node_modules"]);
        assert_eq!(size, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deep_breakdown_has_source() {
        let dir = std::env::temp_dir().join("projector_test_deep");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        let source = dir_size_by_extensions(&dir, &["rs"]);
        assert!(source > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo cooldown test -p projector test_subcmd_size_invalid_format test_dir_size_by_extensions_empty test_dir_size_by_name_found test_dir_size_by_name_not_found test_deep_breakdown 2>&1`
Expected: all pass

- [ ] **Step 5: Register module in `src/subcmd/mod.rs`**

```rust
pub mod size;
```
Add between existing entries (alphabetical: after `search`/`scan`, before `snapshot`).

- [ ] **Step 6: Add `Size` variant in `src/command.rs`**

```rust
    Size {
        path: Option<String>,
        #[arg(long)]
        top: Option<usize>,
        #[arg(long)]
        deep: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
```
Place after `Search` and before `Snapshot` in the `Commands` enum.

- [ ] **Step 7: Add dispatch arm in `src/main.rs`**

```rust
        Commands::Size { path, top, deep, format } => {
            subcmd::size::subcmd_size(path, top, deep, format)?;
            Ok(())
        }
```
Place after the `Search` arm and before `Config`.

- [ ] **Step 8: Build and verify**

Run: `cargo cooldown build 2>&1`
Expected: compiles clean

- [ ] **Step 9: Run full test suite**

Run: `cargo cooldown test 2>&1`
Expected: all tests pass

- [ ] **Step 10: Commit**

```bash
git add src/subcmd/size.rs src/subcmd/mod.rs src/command.rs src/main.rs
git commit -m "feat: add projector size subcommand"
```

---

### Task 3: Create `projector brief` subcommand

**Files:**
- Create: `src/subcmd/brief.rs`
- Modify: `src/subcmd/mod.rs` (add `pub mod brief;`)
- Modify: `src/command.rs` (add `Brief` variant)
- Modify: `src/main.rs` (add dispatch arm)
- Test: inline `#[cfg(test)]` in `src/subcmd/brief.rs`

- [ ] **Step 1: Write failing test skeleton**

```rust
// src/subcmd/brief.rs — initial stub

use anyhow::Result;

pub fn subcmd_brief(days: u32, format: Option<String>) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brief_invalid_format() {
        let result = subcmd_brief(1, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo cooldown test -p projector test_brief_invalid_format 2>&1`
Expected: FAIL

- [ ] **Step 3: Implement `subcmd_brief` in `src/subcmd/brief.rs`**

```rust
use std::path::Path;

use anyhow::Result;
use chrono::Utc;

use crate::analyzer;
use crate::color;
use crate::config::Config;
use crate::snapshot::SnapshotStore;

pub fn subcmd_brief(days: u32, format: Option<String>) -> Result<()> {
    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    let config = Config::load()?;
    let stale_threshold = config.report.stale_threshold_days;

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            println!("{}", color::error("No snapshots found. Run `projector scan` first."));
            return Ok(());
        }
    };

    let total = latest.projects.len();
    let total_loc: u32 = latest.projects.iter().map(|p| p.lines_of_code).sum();

    let high = latest.projects.iter().filter(|p| p.health_score >= 80).count();
    let mid = latest.projects.iter().filter(|p| p.health_score >= 50 && p.health_score < 80).count();
    let low = latest.projects.iter().filter(|p| p.health_score < 50).count();

    let avg_health = if total > 0 {
        latest.projects.iter().map(|p| p.health_score as f64).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let dirty = latest.projects.iter().filter(|p| p.is_dirty).count();

    let now = Utc::now().naive_utc();
    let stale = latest.projects.iter().filter(|p| {
        let days_since = (now - p.last_commit_date).num_days();
        days_since >= stale_threshold as i64
    }).count();

    // Realtime activity: check commits in last N days
    let mut active: Vec<(String, String, u32)> = Vec::new();
    for proj in &latest.projects {
        let dir = Path::new(&proj.path);
        if !dir.exists() {
            continue;
        }
        if let Ok(Some(count)) = analyzer::count_commits_since(dir, days) {
            if count > 0 {
                let name = proj.path.split('/').next_back().unwrap_or(&proj.path).to_string();
                active.push((name, proj.project_type.clone(), count));
            }
        }
    }
    active.sort_by_key(|(_, _, c)| std::cmp::Reverse(*c));

    if fmt == "json" {
        let json = serde_json::json!({
            "date": now.format("%Y-%m-%d").to_string(),
            "total_projects": total,
            "avg_health": format!("{:.1}", avg_health),
            "health_buckets": {
                "good_ge80": high,
                "fair_50_79": mid,
                "poor_lt50": low,
            },
            "total_loc": total_loc,
            "dirty": dirty,
            "stale": stale,
            "active_projects": active.len(),
            "total_commits": active.iter().map(|(_, _, c)| c).sum::<u32>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!();
        println!("  {}", color::info(&format!(
            "Project Brief — {}",
            now.format("%Y-%m-%d")
        )));
        println!("  {}", "═".repeat(40));
        println!();
        println!("  Total:  {} projects", color::cyan(&total.to_string()));
        println!("  Health: {:.1} avg · {} good · {} fair · {} poor", avg_health, high, mid, low);
        println!("  LOC:    {}", total_loc);
        println!("  Dirty:  {} projects", dirty);
        println!("  Stale:  {} projects (>{}d)", stale, stale_threshold);
        println!();

        if !active.is_empty() {
            println!("  {}  Activity (last {}d):", color::info("▸"), days);
            for (name, ptype, count) in &active {
                let type_colored = match *ptype {
                    "Rust" => color::green("Rust"),
                    "JavaScript/TypeScript" => color::yellow("JS"),
                    "Go" => color::blue("Go"),
                    "Python" => color::cyan("Py"),
                    _ => color::white(ptype),
                };
                println!("    {:<20} {:<6} {} commit{}", color::cyan(name), type_colored, count, if *count == 1 { "" } else { "s" });
            }
        } else {
            println!("  No activity in the last {} days.", days);
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brief_invalid_format() {
        let result = subcmd_brief(1, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }

    #[test]
    fn test_brief_graceful_no_snapshot() {
        // Should not panic when no snapshot exists
        let result = subcmd_brief(1, None);
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo cooldown test -p projector test_brief_invalid_format test_brief_graceful_no_snapshot 2>&1`
Expected: all pass

- [ ] **Step 5: Register module in `src/subcmd/mod.rs`**

```rust
pub mod brief;
```
Place alphabetically (after `activity`, before `completion`).

- [ ] **Step 6: Add `Brief` variant in `src/command.rs`**

```rust
    Brief {
        #[arg(long, default_value = "1")]
        days: u32,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
```
Place after `Activity` and before `Completion`.

- [ ] **Step 7: Add dispatch arm in `src/main.rs`**

```rust
        Commands::Brief { days, format } => {
            subcmd::brief::subcmd_brief(days, format)?;
            Ok(())
        }
```

- [ ] **Step 8: Build and verify**

Run: `cargo cooldown build 2>&1`
Expected: compiles clean

- [ ] **Step 9: Run full test suite**

Run: `cargo cooldown test 2>&1`
Expected: all tests pass

- [ ] **Step 10: Commit**

```bash
git add src/subcmd/brief.rs src/subcmd/mod.rs src/command.rs src/main.rs
git commit -m "feat: add projector brief subcommand"
```

---

### Task 4: Create `projector rank` subcommand

**Files:**
- Create: `src/subcmd/rank.rs`
- Modify: `src/subcmd/mod.rs` (add `pub mod rank;`)
- Modify: `src/command.rs` (add `Rank` variant)
- Modify: `src/main.rs` (add dispatch arm)
- Test: inline `#[cfg(test)]` in `src/subcmd/rank.rs`

- [ ] **Step 1: Write failing test skeleton**

```rust
// src/subcmd/rank.rs — initial stub

use anyhow::Result;

pub fn subcmd_rank(
    by: Option<String>,
    reverse: bool,
    type_filter: Option<String>,
    top: Option<usize>,
    category: bool,
    format: Option<String>,
) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_invalid_format() {
        let result = subcmd_rank(None, false, None, None, false, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }

    #[test]
    fn test_rank_invalid_by() {
        let result = subcmd_rank(Some("invalid_metric".to_string()), false, None, None, false, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo cooldown test -p projector test_rank_invalid_format test_rank_invalid_by 2>&1`
Expected: FAIL

- [ ] **Step 3: Implement `subcmd_rank` in `src/subcmd/rank.rs`**

```rust
use std::path::Path;

use anyhow::Result;

use crate::analyzer;
use crate::color;
use crate::snapshot::{ProjectSnapshot, SnapshotStore};

const VALID_METRICS: &[&str] = &["health", "loc", "activity", "age", "commits"];

pub fn subcmd_rank(
    by: Option<String>,
    reverse: bool,
    type_filter: Option<String>,
    top: Option<usize>,
    category: bool,
    format: Option<String>,
) -> Result<()> {
    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    let metric = by.unwrap_or_else(|| "health".to_string());
    if !VALID_METRICS.contains(&metric.as_str()) {
        anyhow::bail!(
            "Invalid metric: '{}'. Valid: {}",
            metric,
            VALID_METRICS.join(", ")
        );
    }

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            println!("{}", crate::color::error("No snapshots found. Run `projector scan` first."));
            return Ok(());
        }
    };

    let mut projects: Vec<(String, String, String, String)> = Vec::new();

    for proj in &latest.projects {
        if let Some(ref t) = type_filter {
            if !proj.project_type.contains(t) {
                continue;
            }
        }

        let name = proj.path.split('/').next_back().unwrap_or(&proj.path);
        let dir = Path::new(&proj.path);

        let value = match metric.as_str() {
            "health" => format!("{}", proj.health_score),
            "loc" => format!("{}", proj.lines_of_code),
            "activity" => {
                let count = if dir.exists() {
                    analyzer::count_commits_since(dir, 30).ok().flatten().unwrap_or(0)
                } else {
                    0
                };
                format!("{}", count)
            }
            "age" => {
                let now = chrono::Utc::now().naive_utc();
                let days = (now - proj.last_commit_date).num_days();
                format!("{}", days)
            }
            "commits" => {
                let count = if dir.exists() {
                    analyzer::count_commits(dir).ok().flatten().map(|s| s.total).unwrap_or(0)
                } else {
                    0
                };
                format!("{}", count)
            }
            _ => unreachable!(),
        };

        projects.push((name.to_string(), proj.project_type.clone(), value, proj.path.clone()));
    }

    // Pad short hex-like values so numeric sort works
    projects.sort_by(|a, b| {
        let a_val: f64 = a.2.parse().unwrap_or(0.0);
        let b_val: f64 = b.2.parse().unwrap_or(0.0);
        if reverse {
            a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            b_val.partial_cmp(&a_val).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    if let Some(n) = top {
        projects.truncate(n);
    }

    if category {
        let mut by_type: std::collections::BTreeMap<String, Vec<(String, String, String, String)>> = std::collections::BTreeMap::new();
        for p in projects {
            by_type.entry(p.1.clone()).or_default().push(p);
        }
        // Only keep #1 per type
        let mut winners: Vec<(String, String, String, String)> = by_type.into_values().filter_map(|v| v.into_iter().next()).collect();
        winners.sort_by(|a, b| {
            let a_val: f64 = a.2.parse().unwrap_or(0.0);
            let b_val: f64 = b.2.parse().unwrap_or(0.0);
            b_val.partial_cmp(&a_val).unwrap_or(std::cmp::Ordering::Equal)
        });
        projects = winners;
    }

    if fmt == "json" {
        let json: Vec<serde_json::Value> = projects.iter().map(|(name, ptype, val, path)| {
            serde_json::json!({
                "name": name,
                "type": ptype,
                "metric": metric,
                "value": val,
                "path": path,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        let metric_upper = match metric.as_str() {
            "health" => "Health".to_string(),
            "loc" => "LOC".to_string(),
            "activity" => format!("Commits (30d)"),
            "age" => "Age (days)".to_string(),
            "commits" => "Total Commits".to_string(),
            _ => metric,
        };

        if category {
            println!();
            println!("  {}  {}", color::info("▸"), color::info(&format!("Per-type leaderboard (by {})", metric_upper)));
            println!();
            for (i, (name, ptype, val, _)) in projects.iter().enumerate() {
                let type_colored = match ptype.as_str() {
                    "Rust" => color::green(ptype),
                    "JavaScript/TypeScript" => color::yellow(ptype),
                    "Go" => color::blue(ptype),
                    "Python" => color::cyan(ptype),
                    _ => color::white(ptype),
                };
                println!("    {:<20}  {:<8}  {:<6}", color::cyan(name), type_colored, color::green(val));
            }
        } else {
            println!();
            println!("  {}  {}", color::info("▸"), color::info(&format!("Project leaderboard (by {})", metric_upper)));
            if reverse {
                println!("  {}", color::info("   (ascending order)"));
            }
            println!();
            let max_name = projects.iter().map(|(n, _, _, _)| n.len()).max().unwrap_or(20).min(40);
            for (i, (name, ptype, val, _)) in projects.iter().enumerate() {
                let rank = format!("{}.", i + 1);
                let type_colored = match ptype.as_str() {
                    "Rust" => color::green(ptype),
                    "JavaScript/TypeScript" => color::yellow(ptype),
                    "Go" => color::blue(ptype),
                    "Python" => color::cyan(ptype),
                    _ => color::white(ptype),
                };
                let val_colored = if val.parse::<f64>().unwrap_or(0.0) > 0.0 {
                    color::green(val)
                } else {
                    color::red(val)
                };
                println!(
                    "  {:>3} {:<max_name$}  {:<8}  {}",
                    color::cyan(&rank),
                    color::blue(name),
                    type_colored,
                    val_colored,
                    max_name = max_name,
                );
            }
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_invalid_format() {
        let result = subcmd_rank(None, false, None, None, false, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }

    #[test]
    fn test_rank_invalid_by() {
        let result = subcmd_rank(Some("invalid_metric".to_string(), false, None, None, false, None));
        assert!(result.is_err());
    }

    #[test]
    fn test_rank_graceful_no_snapshot() {
        let result = subcmd_rank(None, false, None, None, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_metrics_list() {
        assert!(VALID_METRICS.contains(&"health"));
        assert!(VALID_METRICS.contains(&"loc"));
        assert!(VALID_METRICS.contains(&"activity"));
        assert!(VALID_METRICS.contains(&"age"));
        assert!(VALID_METRICS.contains(&"commits"));
        assert_eq!(VALID_METRICS.len(), 5);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo cooldown test -p projector test_rank_invalid_format test_rank_invalid_by test_rank_graceful_no_snapshot test_valid_metrics_list 2>&1`
Expected: all pass

- [ ] **Step 5: Register module in `src/subcmd/mod.rs`**

```rust
pub mod rank;
```
Place alphabetically (after `orphans`, before `report`).

- [ ] **Step 6: Add `Rank` variant in `src/command.rs`**

```rust
    Rank {
        #[arg(long)]
        by: Option<String>,
        #[arg(long)]
        reverse: bool,
        #[arg(long)]
        type_filter: Option<String>,
        #[arg(long)]
        top: Option<usize>,
        #[arg(long)]
        category: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
```
Place after `Orphans` and before `Report`.

Note: clap resolves conflict between `--type` and the existing `Projector` parser context, so use `--type-filter` as the flag name or rename per user preference. Using `#[arg(long = "type")]` via clap.

Actually check: clap derive does NOT conflict — `--type` is a free-standing flag in this context. Let's use `#[arg(long)]` on `type_filter` — clap strips trailing `_filter` and gives `--type-filter`. Or use `#[arg(long = "type")]` for cleaner `--type`.

Use `#[arg(long = "type")]`:

```rust
        #[arg(long = "type")]
        type_filter: Option<String>,
```

- [ ] **Step 7: Add dispatch arm in `src/main.rs`**

```rust
        Commands::Rank { by, reverse, type_filter, top, category, format } => {
            subcmd::rank::subcmd_rank(by, reverse, type_filter, top, category, format)?;
            Ok(())
        }
```

- [ ] **Step 8: Build and verify**

Run: `cargo cooldown build 2>&1`
Expected: compiles clean

- [ ] **Step 9: Run full test suite**

Run: `cargo cooldown test 2>&1`
Expected: all tests pass

- [ ] **Step 10: Commit**

```bash
git add src/subcmd/rank.rs src/subcmd/mod.rs src/command.rs src/main.rs
git commit -m "feat: add projector rank subcommand"
```

---

## Self-Review Checklist

**1. Spec coverage:**
- `projector size` — ✅ Task 1 (utilities) + Task 2 (subcommand): disk usage, `--top`, `--deep`, `--format json`, standalone path arg
- `projector brief` — ✅ Task 3: daily digest with health distribution, activity, stale/dirty counts, `--days`, `--format json`
- `projector rank` — ✅ Task 4: leaderboard with `--by` (health/loc/activity/age/commits), `--reverse`, `--type`, `--top`, `--category`, `--format json`

**2. Placeholder scan:** No TBD, TODO, "implement later", or vague placeholders. Every step has complete code.

**3. Type/method consistency:**
- `human_size(u64) -> String` defined in Task 1, used in Task 2, Task 3, Task 4
- `calc_dir_size(&Path, bool) -> u64` defined in Task 1, used in Task 2
- `subcmd_size`, `subcmd_brief`, `subcmd_rank` signatures match between define and dispatch
- `VALID_METRICS` constant matches switch arms in rank's metric dispatch
- All `--format` handlers use same pattern (check → bail if unknown → json or table)
