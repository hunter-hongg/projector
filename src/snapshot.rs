use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

use crate::config;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSnapshot {
    pub timestamp: NaiveDateTime,
    pub scanned_path: String,
    pub projects: Vec<ProjectSnapshot>,
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
        let dir = Self::snapshots_dir();
        std::fs::create_dir_all(&dir)?;
        let filename = format!("{}.json", snapshot.timestamp.format("%Y%m%d_%H%M%S"));
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(snapshot)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    fn load_by_index(skip: usize) -> Result<Option<ScanSnapshot>> {
        let dir = Self::snapshots_dir();
        if !dir.exists() {
            return Ok(None);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        entries.sort_by_key(|e| e.path());
        if entries.len() <= skip {
            return Ok(None);
        }
        let idx = entries.len() - 1 - skip;
        let content = std::fs::read_to_string(entries[idx].path())?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    pub fn load_latest() -> Result<Option<ScanSnapshot>> {
        Self::load_by_index(0)
    }

    pub fn load_second_latest() -> Result<Option<ScanSnapshot>> {
        Self::load_by_index(1)
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
        }
    }

    fn make_scan(projects: Vec<ProjectSnapshot>) -> ScanSnapshot {
        ScanSnapshot {
            timestamp: Utc::now().naive_utc(),
            scanned_path: ".".to_string(),
            projects,
        }
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
        // removed a (1) + b health_score + b is_dirty (2) + new c (1) = 4
        assert_eq!(diffs.len(), 4);
    }
}
