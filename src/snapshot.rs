use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::config;

/// Current snapshot schema version, written by `save`.
///
/// Bump this whenever a field is added to [`ScanSnapshot`] or
/// [`ProjectSnapshot`]. Older files stay readable because every field carries
/// `#[serde(default)]`; [`migrate`] backfills the fields they are missing.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub path: String,
    pub project_type: String,
    pub git_branch: String,
    pub is_dirty: bool,
    pub unpushed_commits: u32,
    pub last_commit_date: NaiveDateTime,
    pub last_modified_date: NaiveDateTime,
    pub lines_of_code: u32,
    pub health_score: u8,
    /// Depth below the scanned root (0 = direct child). `0` on a snapshot
    /// written before depth was recorded, since legacy scans were one level only.
    #[serde(default)]
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSnapshot {
    pub timestamp: NaiveDateTime,
    pub scanned_path: String,
    pub projects: Vec<ProjectSnapshot>,
    /// `0` means the file predates schema versioning (migratable).
    #[serde(default)]
    pub schema_version: u32,
}

/// Outcome of [`migrate`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MigrateReport {
    /// Files rewritten with the current schema version.
    pub migrated: Vec<PathBuf>,
    /// Files already at [`SCHEMA_VERSION`] (or newer), left untouched.
    pub current: usize,
    /// Files that could not be read or parsed.
    pub failed: Vec<PathBuf>,
}

impl MigrateReport {
    pub fn is_empty(&self) -> bool {
        self.migrated.is_empty() && self.current == 0 && self.failed.is_empty()
    }
}

macro_rules! diff_field {
    ($diffs:ident, $prev:expr, $proj:expr, $field:ident, $variant:expr) => {
        if $prev.$field != $proj.$field {
            $diffs.push(SnapshotDiff {
                path: $proj.path.clone(),
                field: $variant,
                old_value: $prev.$field.to_string(),
                new_value: $proj.$field.to_string(),
            });
        }
    };
}

pub struct SnapshotStore;

impl SnapshotStore {
    fn snapshots_dir() -> PathBuf {
        config::snapshot_dir()
    }

    pub fn save(snapshot: &ScanSnapshot) -> Result<()> {
        Self::save_to(&Self::snapshots_dir(), snapshot)
    }

    fn save_to(dir: &Path, snapshot: &ScanSnapshot) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let filename = format!("{}.json", snapshot.timestamp.format("%Y%m%d_%H%M%S"));
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(snapshot)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn load_by_index(skip: usize) -> Result<Option<ScanSnapshot>> {
        Self::load_by_index_from(&Self::snapshots_dir(), skip)
    }

    fn load_by_index_from(dir: &Path, skip: usize) -> Result<Option<ScanSnapshot>> {
        let entries = Self::sorted_snapshot_files_in(dir)?;
        if entries.len() <= skip {
            return Ok(None);
        }
        let idx = entries.len() - 1 - skip;
        Self::load_file(&entries[idx])
    }

    /// Load one snapshot file, backfilling fields added after it was written.
    ///
    /// A file that cannot be parsed is reported as absent (with a warning)
    /// rather than failing the caller, matching how a missing snapshot is
    /// handled. It is left on disk for `snapshot migrate` to report.
    pub fn load_file(path: &Path) -> Result<Option<ScanSnapshot>> {
        let content = std::fs::read_to_string(path)?;
        let mut snapshot: ScanSnapshot = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: unreadable snapshot '{}': {}", path.display(), e);
                return Ok(None);
            }
        };
        migrate(&mut snapshot);
        Ok(Some(snapshot))
    }

    fn sorted_snapshot_files() -> Result<Vec<PathBuf>> {
        Self::sorted_snapshot_files_in(&Self::snapshots_dir())
    }

    fn sorted_snapshot_files_in(dir: &Path) -> Result<Vec<PathBuf>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .map(|e| e.path())
            .collect();
        entries.sort();
        Ok(entries)
    }

    pub fn load_latest() -> Result<Option<ScanSnapshot>> {
        Self::load_by_index(0)
    }

    pub fn load_second_latest() -> Result<Option<ScanSnapshot>> {
        Self::load_by_index(1)
    }

    pub fn load_all() -> Result<Vec<ScanSnapshot>> {
        Self::load_all_from(&Self::snapshots_dir())
    }

    fn load_all_from(dir: &Path) -> Result<Vec<ScanSnapshot>> {
        let entries = Self::sorted_snapshot_files_in(dir)?;
        let mut snapshots = Vec::with_capacity(entries.len());
        for path in entries {
            if let Some(s) = Self::load_file(&path)? {
                snapshots.push(s);
            }
        }
        Ok(snapshots)
    }

    /// Rewrite every legacy snapshot on disk at [`SCHEMA_VERSION`].
    ///
    /// The on-disk version is read before the in-memory backfill runs, so
    /// files that only `load_file` would patch are still detected here.
    pub fn migrate_all() -> Result<MigrateReport> {
        Self::migrate_all_from(&Self::snapshots_dir())
    }

    fn migrate_all_from(dir: &Path) -> Result<MigrateReport> {
        let mut report = MigrateReport::default();
        for path in Self::sorted_snapshot_files_in(dir)? {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Warning: cannot read '{}': {}", path.display(), e);
                    report.failed.push(path);
                    continue;
                }
            };
            let mut snapshot: ScanSnapshot = match serde_json::from_str(&content) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Warning: corrupt snapshot '{}': {}", path.display(), e);
                    report.failed.push(path);
                    continue;
                }
            };
            if snapshot.schema_version >= SCHEMA_VERSION {
                report.current += 1;
                continue;
            }
            migrate(&mut snapshot);
            snapshot.schema_version = SCHEMA_VERSION;
            let json = serde_json::to_string_pretty(&snapshot)?;
            std::fs::write(&path, json)?;
            report.migrated.push(path);
        }
        Ok(report)
    }

    pub fn prune(keep: u32, dry_run: bool) -> Result<Vec<PathBuf>> {
        let entries = Self::sorted_snapshot_files()?;
        if keep as usize >= entries.len() {
            return Ok(Vec::new());
        }
        let to_remove: Vec<_> = entries
            .iter()
            .take(entries.len() - keep as usize)
            .cloned()
            .collect();
        if !dry_run {
            for path in &to_remove {
                std::fs::remove_file(path)?;
            }
        }
        Ok(to_remove)
    }

    pub fn diff(latest: &ScanSnapshot, previous: &ScanSnapshot) -> Vec<SnapshotDiff> {
        let mut diffs = Vec::new();
        for proj in &latest.projects {
            let prev = previous.projects.iter().find(|p| p.path == proj.path);
            match prev {
                Some(prev) => {
                    diff_field!(diffs, prev, proj, health_score, DiffField::HealthScore);
                    diff_field!(diffs, prev, proj, is_dirty, DiffField::IsDirty);
                    diff_field!(diffs, prev, proj, lines_of_code, DiffField::LinesOfCode);
                    diff_field!(
                        diffs,
                        prev,
                        proj,
                        unpushed_commits,
                        DiffField::UnpushedCommits
                    );
                    diff_field!(diffs, prev, proj, git_branch, DiffField::GitBranch);
                    diff_field!(diffs, prev, proj, project_type, DiffField::ProjectType);
                }
                None => {
                    diffs.push(SnapshotDiff {
                        path: proj.path.clone(),
                        field: DiffField::Project,
                        old_value: String::new(),
                        new_value: "new".to_string(),
                    });
                }
            }
        }
        for prev in &previous.projects {
            if !latest.projects.iter().any(|p| p.path == prev.path) {
                diffs.push(SnapshotDiff {
                    path: prev.path.clone(),
                    field: DiffField::Project,
                    old_value: "removed".to_string(),
                    new_value: String::new(),
                });
            }
        }
        diffs
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffField {
    HealthScore,
    IsDirty,
    LinesOfCode,
    UnpushedCommits,
    GitBranch,
    ProjectType,
    Project,
}

impl fmt::Display for DiffField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffField::HealthScore => write!(f, "health_score"),
            DiffField::IsDirty => write!(f, "is_dirty"),
            DiffField::LinesOfCode => write!(f, "lines_of_code"),
            DiffField::UnpushedCommits => write!(f, "unpushed_commits"),
            DiffField::GitBranch => write!(f, "git_branch"),
            DiffField::ProjectType => write!(f, "project_type"),
            DiffField::Project => write!(f, "project"),
        }
    }
}

pub struct SnapshotDiff {
    pub path: String,
    pub field: DiffField,
    pub old_value: String,
    pub new_value: String,
}

/// Backfill fields missing from a snapshot written by an older projector.
///
/// Returns `true` when the snapshot was changed. Runs in memory on every
/// load, so old files stay readable even before `snapshot migrate` runs.
pub fn migrate(snapshot: &mut ScanSnapshot) -> bool {
    if snapshot.schema_version >= SCHEMA_VERSION {
        return false;
    }
    // Version 0 predates `depth`: every legacy scan walked one level, so every
    // recorded project was a direct child of the scanned root.
    for project in &mut snapshot.projects {
        project.depth = project.depth.max(1);
    }
    snapshot.schema_version = SCHEMA_VERSION;
    true
}

pub fn format_health_deductions(
    is_dirty: bool,
    last_commit_date: NaiveDateTime,
    lines_of_code: u32,
    stale_threshold_days: u32,
) -> Vec<String> {
    use chrono::Utc;
    let now = Utc::now().naive_utc();
    let mut reasons = Vec::new();

    let days_since_commit = (now - last_commit_date).num_days();
    if days_since_commit >= stale_threshold_days as i64 {
        reasons.push(format!("stale({}d)", days_since_commit));
    }

    if is_dirty {
        reasons.push("dirty".to_string());
    }

    if lines_of_code < 100 {
        reasons.push("loc<100".to_string());
    }

    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_project(
        path: &str,
        health: u8,
        dirty: bool,
        loc: u32,
        unpushed: u32,
    ) -> ProjectSnapshot {
        let now = Utc::now().naive_utc();
        ProjectSnapshot {
            path: path.to_string(),
            project_type: "Rust".to_string(),
            git_branch: "main".to_string(),
            is_dirty: dirty,
            unpushed_commits: unpushed,
            last_commit_date: now,
            last_modified_date: now,
            lines_of_code: loc,
            health_score: health,
            depth: 1,
        }
    }

    fn make_scan(projects: Vec<ProjectSnapshot>) -> ScanSnapshot {
        ScanSnapshot {
            timestamp: Utc::now().naive_utc(),
            scanned_path: ".".to_string(),
            projects,
            schema_version: SCHEMA_VERSION,
        }
    }

    #[test]
    fn test_legacy_snapshot_without_version_or_depth_deserializes() {
        let legacy = r#"{
            "timestamp": "2026-01-01T00:00:00",
            "scanned_path": "/home/me/projects",
            "projects": [{
                "path": "/home/me/projects/app",
                "project_type": "Rust",
                "git_branch": "main",
                "is_dirty": false,
                "unpushed_commits": 0,
                "last_commit_date": "2026-01-01T00:00:00",
                "last_modified_date": "2026-01-01T00:00:00",
                "lines_of_code": 100,
                "health_score": 100
            }]
        }"#;
        let mut snapshot: ScanSnapshot = serde_json::from_str(legacy).unwrap();
        assert_eq!(snapshot.schema_version, 0);
        assert_eq!(snapshot.projects[0].depth, 0);
        assert!(migrate(&mut snapshot));
        assert_eq!(snapshot.schema_version, SCHEMA_VERSION);
        assert_eq!(snapshot.projects[0].depth, 1);
    }

    #[test]
    fn test_migrate_is_idempotent() {
        let mut snapshot = make_scan(vec![make_project("proj_a", 100, false, 500, 0)]);
        assert!(!migrate(&mut snapshot));
        assert_eq!(snapshot.projects[0].depth, 1);
    }

    #[test]
    fn test_migrate_does_not_clobber_real_depth() {
        let mut snapshot = make_scan(vec![make_project("proj_a", 100, false, 500, 0)]);
        snapshot.projects[0].depth = 3;
        snapshot.schema_version = 0;
        assert!(migrate(&mut snapshot));
        assert_eq!(snapshot.projects[0].depth, 3);
    }

    #[test]
    fn test_save_roundtrip_keeps_schema_version() {
        let mut snapshot = make_scan(vec![make_project("proj_a", 100, false, 500, 0)]);
        snapshot.projects[0].depth = 2;
        let json = serde_json::to_string(&snapshot).unwrap();
        let mut back: ScanSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, SCHEMA_VERSION);
        assert!(!migrate(&mut back));
        assert_eq!(back.projects[0].depth, 2);
    }

    #[test]
    fn test_migrate_report_is_empty() {
        assert!(MigrateReport::default().is_empty());
        assert!(
            !MigrateReport {
                current: 1,
                ..Default::default()
            }
            .is_empty()
        );
    }

    #[test]
    fn test_diff_new_project() {
        let a = make_project("proj_a", 100, false, 500, 0);
        let latest = make_scan(vec![a]);
        let prev = make_scan(vec![]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, DiffField::Project);
        assert_eq!(diffs[0].new_value, "new");
        assert!(diffs[0].old_value.is_empty());
    }

    #[test]
    fn test_diff_removed_project() {
        let a = make_project("proj_a", 100, false, 500, 0);
        let latest = make_scan(vec![]);
        let prev = make_scan(vec![a]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, DiffField::Project);
        assert_eq!(diffs[0].old_value, "removed");
        assert!(diffs[0].new_value.is_empty());
    }

    #[test]
    fn test_diff_health_change() {
        let a_old = make_project("proj_a", 80, false, 500, 0);
        let a_new = make_project("proj_a", 100, false, 500, 0);
        let latest = make_scan(vec![a_new]);
        let prev = make_scan(vec![a_old]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, DiffField::HealthScore);
        assert_eq!(diffs[0].old_value, "80");
        assert_eq!(diffs[0].new_value, "100");
    }

    #[test]
    fn test_diff_dirty_change() {
        let a_old = make_project("proj_a", 100, false, 500, 0);
        let a_new = make_project("proj_a", 90, true, 500, 0);
        let latest = make_scan(vec![a_new]);
        let prev = make_scan(vec![a_old]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert_eq!(diffs.len(), 2);
        assert!(diffs.iter().any(|d| d.field == DiffField::IsDirty));
        assert!(diffs.iter().any(|d| d.field == DiffField::HealthScore));
    }

    #[test]
    fn test_diff_loc_change() {
        let a_old = make_project("proj_a", 100, false, 500, 0);
        let a_new = make_project("proj_a", 100, false, 600, 0);
        let latest = make_scan(vec![a_new]);
        let prev = make_scan(vec![a_old]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, DiffField::LinesOfCode);
    }

    #[test]
    fn test_diff_no_changes() {
        let a = make_project("proj_a", 100, false, 500, 0);
        let latest = make_scan(vec![a.clone()]);
        let prev = make_scan(vec![a]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_multiple_projects() {
        let a = make_project("proj_a", 100, false, 500, 0);
        let b = make_project("proj_b", 80, true, 200, 3);
        let c = make_project("proj_c", 70, false, 100, 0);
        let prev = make_scan(vec![a, b]);
        let new_b = make_project("proj_b", 85, false, 200, 3);
        let latest = make_scan(vec![new_b, c]);
        let diffs = SnapshotStore::diff(&latest, &prev);
        assert_eq!(diffs.len(), 4);
    }

    #[test]
    fn test_format_health_deductions_clean() {
        let now = Utc::now().naive_utc();
        let reasons = format_health_deductions(false, now, 500, 90);
        assert!(reasons.is_empty());
    }

    #[test]
    fn test_format_health_deductions_dirty() {
        let now = Utc::now().naive_utc();
        let reasons = format_health_deductions(true, now, 500, 90);
        assert!(reasons.contains(&"dirty".to_string()));
    }

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("projector_test_migrate_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const LEGACY_JSON: &str = r#"{
        "timestamp": "2026-01-01T00:00:00",
        "scanned_path": "/home/me/projects",
        "projects": [{
            "path": "/home/me/projects/app",
            "project_type": "Rust",
            "git_branch": "main",
            "is_dirty": false,
            "unpushed_commits": 0,
            "last_commit_date": "2026-01-01T00:00:00",
            "last_modified_date": "2026-01-01T00:00:00",
            "lines_of_code": 100,
            "health_score": 100
        }]
    }"#;

    #[test]
    fn test_migrate_all_rewrites_legacy_file_on_disk() {
        let dir = scratch_dir("rewrite");
        std::fs::write(dir.join("20260101_000000.json"), LEGACY_JSON).unwrap();

        let report = SnapshotStore::migrate_all_from(&dir).unwrap();
        assert_eq!(report.migrated.len(), 1);
        assert_eq!(report.current, 0);
        assert!(report.failed.is_empty());

        let on_disk = std::fs::read_to_string(dir.join("20260101_000000.json")).unwrap();
        let after: ScanSnapshot = serde_json::from_str(&on_disk).unwrap();
        assert_eq!(after.schema_version, SCHEMA_VERSION);
        assert_eq!(after.projects[0].depth, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_migrate_all_is_idempotent() {
        let dir = scratch_dir("idempotent");
        std::fs::write(dir.join("20260101_000000.json"), LEGACY_JSON).unwrap();

        assert_eq!(
            SnapshotStore::migrate_all_from(&dir)
                .unwrap()
                .migrated
                .len(),
            1
        );
        let second = SnapshotStore::migrate_all_from(&dir).unwrap();
        assert!(second.migrated.is_empty());
        assert_eq!(second.current, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_migrate_all_reports_corrupt_file_without_deleting_it() {
        let dir = scratch_dir("corrupt");
        std::fs::write(dir.join("20260101_000001.json"), "{ not json").unwrap();
        std::fs::write(dir.join("20260101_000002.json"), LEGACY_JSON).unwrap();

        let report = SnapshotStore::migrate_all_from(&dir).unwrap();
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.migrated.len(), 1);
        assert!(
            dir.join("20260101_000001.json").exists(),
            "a corrupt snapshot must not be deleted by a migration"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_migrate_all_empty_dir() {
        let dir = scratch_dir("empty");
        let report = SnapshotStore::migrate_all_from(&dir).unwrap();
        assert!(report.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_file_backfills_without_touching_disk() {
        let dir = scratch_dir("load_backfill");
        let path = dir.join("20260101_000000.json");
        std::fs::write(&path, LEGACY_JSON).unwrap();

        let loaded = SnapshotStore::load_file(&path)
            .unwrap()
            .expect("legacy file loads");
        assert_eq!(loaded.projects[0].depth, 1);

        // Reading must not have rewritten the file.
        let raw = std::fs::read_to_string(&path).unwrap();
        let untouched: ScanSnapshot = serde_json::from_str(&raw).unwrap();
        assert_eq!(untouched.schema_version, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_file_corrupt_returns_none_not_error() {
        let dir = scratch_dir("load_corrupt");
        let path = dir.join("20260101_000000.json");
        std::fs::write(&path, "{ broken").unwrap();

        assert!(SnapshotStore::load_file(&path).unwrap().is_none());
        assert!(path.exists(), "corrupt file is left for the user");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_to_roundtrips_new_schema() {
        let dir = scratch_dir("save_to");
        let mut snapshot = make_scan(vec![make_project("proj_a", 100, false, 500, 0)]);
        snapshot.projects[0].depth = 4;
        SnapshotStore::save_to(&dir, &snapshot).unwrap();

        let reloaded = SnapshotStore::load_by_index_from(&dir, 0)
            .unwrap()
            .expect("saved snapshot loads");
        assert_eq!(reloaded.projects[0].depth, 4);
        assert_eq!(reloaded.schema_version, SCHEMA_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
