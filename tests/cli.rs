//! End-to-end CLI integration tests.
//!
//! Each test runs the real `projector` binary as a subprocess with `HOME`
//! pointed at a private temp directory, so `~/.projector` (config, snapshots,
//! tags) never touches the developer's real one and tests cannot collide.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Path to the built `projector` binary (set by Cargo for integration tests).
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_projector")
}

/// Fresh temp HOME + scratch dir, owned by this test only.
fn scratch(test: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "projector_it_{}_{}_{}",
        test,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(base.join("home")).unwrap();
    base
}

/// Run `projector <args>` with `HOME` set to the test home.
fn projector(home: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .env("HOME", home)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// Create a real git repo with one commit at `dir`.
fn git_repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir)
            .status()
            .unwrap()
            .success(),
        "git init failed — is git installed?"
    );
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ])
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git commit failed");
}

fn snapshots_dir(home: &Path) -> PathBuf {
    home.join(".projector").join("snapshots")
}

#[test]
fn scan_writes_versioned_snapshot_and_report_reads_it() {
    let root = scratch("scan_snapshot");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("app-a"));
    git_repo(&work.join("app-b"));

    let out = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    assert!(stdout(&out).contains("Scanned 2 projects"));

    // One JSON snapshot landed in the isolated ~/.projector.
    let snap = snapshots_dir(&home);
    let files: Vec<_> = std::fs::read_dir(&snap).unwrap().collect();
    assert_eq!(files.len(), 1, "expected exactly one snapshot file");
    let content = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["projects"].as_array().unwrap().len(), 2);
    assert_eq!(json["projects"][0]["depth"], 1);

    // report -f json parses and lists both projects.
    let rep = projector(&home, &["report", "-f", "json"]);
    assert!(rep.status.success(), "report failed: {}", stderr(&rep));
    let rep_json: serde_json::Value = serde_json::from_str(&stdout(&rep)).unwrap();
    assert_eq!(rep_json["projects"].as_array().unwrap().len(), 2);

    // report -f md renders a table.
    let rep_md = projector(&home, &["report", "-f", "md"]);
    assert!(
        rep_md.status.success(),
        "report -f md failed: {}",
        stderr(&rep_md)
    );
    assert!(stdout(&rep_md).contains("Project Health Report"));
    assert!(stdout(&rep_md).contains("| Project |"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn nested_repos_discovered_via_config_max_depth() {
    let root = scratch("nestdepth");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("mono").join("inner")); // depth 2: mono/inner
    git_repo(&work.join("top")); // depth 1

    // Default max_depth = 1: only the direct child is found.
    let out = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("Scanned 1 projects"),
        "{}",
        stdout(&out)
    );

    // Raise the cap through the config subcommand, rescan.
    let cfg = projector(&home, &["config", "set", "scan.max_depth", "2"]);
    assert!(cfg.status.success(), "config set failed: {}", stderr(&cfg));

    let out = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(out.status.success(), "scan failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("Scanned 2 projects"),
        "{}",
        stdout(&out)
    );

    let snap = snapshots_dir(&home);
    let files: Vec<_> = std::fs::read_dir(&snap).unwrap().collect();
    let content = std::fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let depths: Vec<u32> = json["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["depth"].as_u64().unwrap() as u32)
        .collect();
    assert!(depths.contains(&1));
    assert!(
        depths.contains(&2),
        "nested repo at depth 2 must be discovered"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn snapshot_migrate_rewrites_legacy_file() {
    let root = scratch("migrate");
    let home = root.join("home");
    let dir = snapshots_dir(&home);
    std::fs::create_dir_all(&dir).unwrap();

    // A snapshot as an older projector wrote it: no schema_version, no depth.
    let legacy = serde_json::json!({
        "timestamp": "2026-01-02T03:04:05",
        "scanned_path": "/tmp/x",
        "projects": [
            {
                "path": "/tmp/x/app",
                "project_type": "Rust",
                "git_branch": "main",
                "is_dirty": false,
                "unpushed_commits": 0,
                "last_commit_date": "2026-01-01T00:00:00",
                "last_modified_date": "2026-01-01T00:00:00",
                "lines_of_code": 10,
                "health_score": 80
            }
        ]
    });
    std::fs::write(
        dir.join("20260102_030405.json"),
        serde_json::to_string(&legacy).unwrap(),
    )
    .unwrap();

    let out = projector(&home, &["snapshot", "migrate"]);
    assert!(out.status.success(), "migrate failed: {}", stderr(&out));
    assert!(
        stdout(&out).contains("Migrated 1 snapshot(s)"),
        "{}",
        stdout(&out)
    );

    let content = std::fs::read_to_string(dir.join("20260102_030405.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(
        json["projects"][0]["depth"], 1,
        "legacy direct-child depth backfilled to 1"
    );

    // Idempotent second run.
    let again = projector(&home, &["snapshot", "migrate"]);
    assert!(again.status.success());
    assert!(
        stdout(&again).contains("already at schema version"),
        "{}",
        stdout(&again)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn format_flag_rejects_unsupported_values() {
    let root = scratch("format");
    let home = root.join("home");

    for (args, wanted) in [
        (vec!["deps", "-f", "md"], "terminal' or 'json'"),
        (vec!["stats", "-f", "yaml"], "terminal' or 'json'"),
        (vec!["search", "-f", "xml", "rust"], "terminal' or 'json'"),
    ] {
        let out = projector(&home, &args);
        assert!(!out.status.success(), "{args:?} should have failed");
        let msg = stdout(&out) + &stderr(&out);
        assert!(msg.contains("Unsupported format"), "{args:?}: {msg}");
        assert!(msg.contains(wanted), "{args:?}: {msg}");
    }

    // report still accepts both json and md.
    let rep = projector(&home, &["report", "-f", "md"]);
    assert!(
        rep.status.success(),
        "report -f md unacceptable: {}",
        stderr(&rep)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn version_and_help_smoke() {
    let root = scratch("smoke");
    let home = root.join("home");

    let v = projector(&home, &["--version"]);
    assert!(v.status.success());
    assert!(
        stdout(&v).starts_with("projector "),
        "unexpected version output"
    );

    let h = projector(&home, &["--help"]);
    assert!(h.status.success());
    assert!(stdout(&h).contains("Usage:"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tag_lifecycle_via_cli() {
    let root = scratch("tag");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("tagged-a"));
    git_repo(&work.join("tagged-b"));

    // scan first so the projects exist in a snapshot (tagging itself only
    // needs a path string, but this exercises scan + tag together).
    let scan = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(scan.status.success(), "scan failed: {}", stderr(&scan));

    // tag set / list / rm / clear
    let set = projector(
        &home,
        &["tag", "set", work.join("tagged-a").to_str().unwrap(), "go"],
    );
    assert!(set.status.success(), "tag set failed: {}", stderr(&set));
    assert!(stdout(&set).contains("Tagged"));

    let list = projector(&home, &["tag", "list"]);
    assert!(list.status.success(), "tag list failed: {}", stderr(&list));
    assert!(stdout(&list).contains("go"));

    let tagged = projector(
        &home,
        &["tag", "list", work.join("tagged-a").to_str().unwrap()],
    );
    assert!(tagged.status.success());
    assert!(stdout(&tagged).contains("go"));

    let rm = projector(
        &home,
        &["tag", "rm", work.join("tagged-a").to_str().unwrap(), "go"],
    );
    assert!(rm.status.success());
    assert!(stdout(&rm).contains("Removed tag"));

    let cleared = projector(
        &home,
        &["tag", "clear", work.join("tagged-a").to_str().unwrap()],
    );
    assert!(cleared.status.success());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn export_html_and_markdown_via_cli() {
    let root = scratch("export");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("app-x"));
    git_repo(&work.join("app-y"));

    let scan = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(scan.status.success(), "scan failed: {}", stderr(&scan));

    // HTML to stdout contains the dashboard doctype marker.
    let html = projector(&home, &["export", "html"]);
    assert!(
        html.status.success(),
        "export html failed: {}",
        stderr(&html)
    );
    assert!(stdout(&html).contains("Projector Dashboard"));

    // HTML to a file lands on disk.
    let html_file = root.join("dash.html");
    let html_out = projector(
        &home,
        &["export", "html", "-o", html_file.to_str().unwrap()],
    );
    assert!(html_out.status.success(), "{}", stderr(&html_out));
    assert!(html_file.exists());
    assert!(
        std::fs::read_to_string(&html_file)
            .unwrap()
            .contains("Projector Dashboard")
    );

    // Markdown to stdout.
    let md = projector(&home, &["export", "markdown"]);
    assert!(
        md.status.success(),
        "export markdown failed: {}",
        stderr(&md)
    );
    assert!(stdout(&md).contains("# Projector Dashboard"));
    assert!(stdout(&md).contains("| Project | Type |"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn completion_emits_valid_for_all_shells() {
    let root = scratch("completion");
    let home = root.join("home");

    for shell in &["bash", "zsh", "fish"] {
        let out = projector(&home, &["completion", shell]);
        // clap_complete prints to stdout and exits 0.
        assert!(
            out.status.success(),
            "completion {shell} failed: {}",
            stderr(&out)
        );
        let text = stdout(&out);
        assert!(!text.is_empty(), "completion {shell} was empty");
        // Each generator references the CLI name.
        assert!(
            text.to_lowercase().contains("projector"),
            "completion {shell} did not reference projector"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn snapshot_prune_keeps_latest() {
    let root = scratch("prune");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("app"));

    // Snapshot filenames are second-resolution (`YYYYMMDD_HHMMSS.json`), so
    // two `scan` calls within the same second collide — write a second snapshot
    // directly under an earlier name to model the on-disk state `prune` reads.
    let scan = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(scan.status.success(), "scan failed: {}", stderr(&scan));

    let dir = snapshots_dir(&home);
    let fresh: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
    assert_eq!(fresh.len(), 1, "scan should write exactly one snapshot");
    let fresh_src = fresh[0].as_ref().unwrap().path();
    let legacy_dst = dir.join("20200101_000000.json");
    std::fs::copy(&fresh_src, &legacy_dst).unwrap();

    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        2,
        "expected two snapshots before pruning"
    );

    // Dry run reports but does not delete.
    let dry = projector(&home, &["snapshot", "prune", "--keep", "1", "--dry-run"]);
    assert!(dry.status.success(), "{}", stderr(&dry));
    assert!(stdout(&dry).contains("Would remove"));
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        2,
        "dry run must not delete"
    );

    // Real prune keeps exactly `--keep`.
    let real = projector(&home, &["snapshot", "prune", "--keep", "1"]);
    assert!(real.status.success(), "{}", stderr(&real));
    assert!(stdout(&real).contains("Removed 1 snapshot"));
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "pruning must leave exactly the kept count"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn stats_emits_json_and_counts_projects() {
    let root = scratch("stats");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("a"));
    git_repo(&work.join("b"));

    let scan = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(scan.status.success(), "{}", stderr(&scan));

    let stats = projector(&home, &["stats", "-f", "json"]);
    assert!(stats.status.success(), "{}", stderr(&stats));
    let json: serde_json::Value = serde_json::from_str(&stdout(&stats)).unwrap();
    assert_eq!(json["total_projects"], 2);
    assert!(json["avg_health"].as_f64().is_some());
    assert!(json["health_buckets"]["high_ge80"].as_u64().is_some());

    // Without -f json, the terminal table still renders.
    let term = projector(&home, &["stats"]);
    assert!(term.status.success(), "{}", stderr(&term));
    assert!(stdout(&term).contains("Global project statistics"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn list_json_and_filters_via_cli() {
    let root = scratch("list_filter");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("rust-app"));
    git_repo(&work.join("mono").join("inner"));
    std::fs::write(work.join("rust-app").join("Cargo.toml"), "").unwrap();

    // The default max_depth is 1; raise it so the nested mono/inner repo is
    // discovered at all (mirrors nested_repos_discovered_via_config_max_depth).
    let cfg = projector(&home, &["config", "set", "scan.max_depth", "2"]);
    assert!(cfg.status.success(), "config set failed: {}", stderr(&cfg));

    // Plain `list` shows every discovered project with its depth.
    let out = projector(&home, &["list", work.to_str().unwrap()]);
    assert!(out.status.success(), "list failed: {}", stderr(&out));
    assert!(stdout(&out).contains("rust-app"));
    assert!(stdout(&out).contains("depth 1"));

    // `--type` filters by project-type substring (like `rank --type`).
    let typed = projector(&home, &["list", "--type", "Rust", work.to_str().unwrap()]);
    assert!(typed.status.success(), "{}", stderr(&typed));
    let t = stdout(&typed);
    assert!(t.contains("rust-app"));
    assert!(
        !t.contains("inner"),
        "--type Rust must drop the unknown mono/inner"
    );

    // `--depth N` keeps only projects found at exactly that depth.
    let deep = projector(&home, &["list", "--depth", "2", work.to_str().unwrap()]);
    assert!(deep.status.success(), "{}", stderr(&deep));
    assert!(stdout(&deep).contains("inner"));
    assert!(!stdout(&deep).contains("rust-app"));

    // `-f json` emits structured rows (path, type, depth, tags).
    let json = projector(&home, &["list", "-f", "json", work.to_str().unwrap()]);
    assert!(json.status.success(), "{}", stderr(&json));
    let rows: serde_json::Value = serde_json::from_str(&stdout(&json)).unwrap();
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 2, "json must list both projects: {rows}");
    let depths: Vec<u64> = arr.iter().map(|r| r["depth"].as_u64().unwrap()).collect();
    assert!(depths.contains(&1));
    assert!(depths.contains(&2));
    assert!(
        arr.iter().any(|r| r["project_type"] == "Rust"),
        "Cargo.toml project must detect as Rust"
    );
    assert!(
        arr.iter().all(|r| r["tags"].as_array().unwrap().is_empty()),
        "no tags were set, so tags must be empty"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn trend_metrics_and_markdown_via_cli() {
    let root = scratch("trend_metrics");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("app"));

    // Two snapshots with distinct timestamps (filenames are second-resolution).
    let s1 = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(s1.status.success(), "{}", stderr(&s1));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let s2 = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(s2.status.success(), "{}", stderr(&s2));

    // New metrics are accepted and produce a data point per snapshot.
    for metric in ["health", "loc", "unpushed", "dirty", "age", "projects"] {
        let out = projector(&home, &["trend", "--metric", metric, "-f", "json"]);
        assert!(
            out.status.success(),
            "trend --metric {metric} failed: {}",
            stderr(&out)
        );
        let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2, "{metric}: expected 2 snapshots");
        assert_eq!(arr[0]["metric"], metric);
        assert!(arr[0]["value"].as_f64().is_some());
    }

    // An unknown metric is rejected with the valid list.
    let bad = projector(&home, &["trend", "--metric", "nope"]);
    assert!(!bad.status.success());
    let msg = stdout(&bad) + &stderr(&bad);
    assert!(msg.contains("Invalid metric"), "{msg}");
    assert!(
        msg.contains("health, loc, unpushed, dirty, age, projects"),
        "{msg}"
    );

    // Markdown output renders a table.
    let md = projector(&home, &["trend", "-f", "md"]);
    assert!(md.status.success(), "{}", stderr(&md));
    let text = stdout(&md);
    assert!(text.contains("# Health Score Trend"));
    assert!(text.contains("| Date |"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn deps_outdated_is_offline_without_the_flag() {
    let root = scratch("deps_noflag");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("a"));
    // A real package.json so npm could be consulted, but the flag is absent.
    std::fs::write(
        work.join("a").join("package.json"),
        r#"{"dependencies":{"lodash":"*"}}"#,
    )
    .unwrap();

    let scan = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(scan.status.success(), "{}", stderr(&scan));

    // No --outdated: the deps JSON must not contain an `outdated` block at all,
    // proving we did not shell out to any registry tool.
    let deps = projector(&home, &["deps", "-f", "json"]);
    assert!(deps.status.success(), "{}", stderr(&deps));
    let json: serde_json::Value = serde_json::from_str(&stdout(&deps)).unwrap();
    // `deps` JSON without `--outdated` sets the key to `null`, not omitting it.
    assert!(
        json.get("outdated")
            .unwrap_or(&serde_json::Value::Null)
            .is_null(),
        "deps without --outdated must not invoke outdated checks: {json}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn brief_markdown_is_pastable_digested() {
    let root = scratch("brief_md");
    let home = root.join("home");
    let work = root.join("work");
    git_repo(&work.join("app"));

    let scan = projector(&home, &["scan", work.to_str().unwrap()]);
    assert!(scan.status.success(), "{}", stderr(&scan));

    let md = projector(&home, &["brief", "-f", "md"]);
    assert!(md.status.success(), "{}", stderr(&md));
    let text = stdout(&md);
    assert!(text.contains("# Project Brief"));
    assert!(text.contains("| Metric | Value |"));
    assert!(text.contains("| Project | Type | Commits |"));

    let _ = std::fs::remove_dir_all(&root);
}
