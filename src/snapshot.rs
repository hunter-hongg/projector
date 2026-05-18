use anyhow::Result;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
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

pub struct SnapshotStore;

impl SnapshotStore {
    fn snapshots_dir() -> PathBuf {
        config::snapshot_dir()
    }

    pub fn save(snapshot: &ScanSnapshot) -> Result<()> {
        let dir = Self::snapshots_dir();
        let filename = format!("{}.json", snapshot.timestamp.format("%Y%m%d_%H%M%S"));
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(snapshot)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    pub fn load_latest() -> Result<Option<ScanSnapshot>> {
        let dir = Self::snapshots_dir();
        if !dir.exists() {
            return Ok(None);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .collect();
        entries.sort_by_key(|e| e.path());
        if let Some(latest) = entries.last() {
            let content = std::fs::read_to_string(latest.path())?;
            Ok(Some(serde_json::from_str(&content)?))
        } else {
            Ok(None)
        }
    }

    pub fn load_second_latest() -> Result<Option<ScanSnapshot>> {
        let dir = Self::snapshots_dir();
        if !dir.exists() {
            return Ok(None);
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .collect();
        entries.sort_by_key(|e| e.path());
        if entries.len() >= 2 {
            let content = std::fs::read_to_string(entries[entries.len() - 2].path())?;
            Ok(Some(serde_json::from_str(&content)?))
        } else {
            Ok(None)
        }
    }

    pub fn diff(latest: &ScanSnapshot, previous: &ScanSnapshot) -> Vec<SnapshotDiff> {
        let mut diffs = Vec::new();
        for proj in &latest.projects {
            let prev = previous.projects.iter().find(|p| p.path == proj.path);
            match prev {
                Some(prev) => {
                    if prev.health_score != proj.health_score {
                        diffs.push(SnapshotDiff {
                            path: proj.path.clone(),
                            field: "health_score".to_string(),
                            old_value: prev.health_score.to_string(),
                            new_value: proj.health_score.to_string(),
                        });
                    }
                    if prev.is_dirty != proj.is_dirty {
                        diffs.push(SnapshotDiff {
                            path: proj.path.clone(),
                            field: "is_dirty".to_string(),
                            old_value: prev.is_dirty.to_string(),
                            new_value: proj.is_dirty.to_string(),
                        });
                    }
                    if prev.lines_of_code != proj.lines_of_code {
                        diffs.push(SnapshotDiff {
                            path: proj.path.clone(),
                            field: "lines_of_code".to_string(),
                            old_value: prev.lines_of_code.to_string(),
                            new_value: proj.lines_of_code.to_string(),
                        });
                    }
                    if prev.unpushed_commits != proj.unpushed_commits {
                        diffs.push(SnapshotDiff {
                            path: proj.path.clone(),
                            field: "unpushed_commits".to_string(),
                            old_value: prev.unpushed_commits.to_string(),
                            new_value: proj.unpushed_commits.to_string(),
                        });
                    }
                }
                None => {
                    diffs.push(SnapshotDiff {
                        path: proj.path.clone(),
                        field: "project".to_string(),
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
                    field: "project".to_string(),
                    old_value: "removed".to_string(),
                    new_value: String::new(),
                });
            }
        }
        diffs
    }
}

pub struct SnapshotDiff {
    pub path: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}
