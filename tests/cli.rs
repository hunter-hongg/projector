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
