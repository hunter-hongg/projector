use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::{NaiveDateTime, TimeZone, Utc};

use crate::detect::ProjectType;
use crate::git::git_health;
use crate::metrics::estimate_loc;
use crate::snapshot::ProjectSnapshot;

pub fn compute_health_score(
    is_dirty: bool,
    unpushed_commits: u32,
    last_commit_date: NaiveDateTime,
    last_modified_date: NaiveDateTime,
    lines_of_code: u32,
    stale_threshold_days: u32,
) -> u8 {
    let now = Utc::now().naive_utc();
    let mut score: i32 = 100;

    let days_since_commit = (now - last_commit_date).num_days();
    if days_since_commit >= stale_threshold_days as i64 {
        score -= 15;
    }

    if is_dirty {
        score -= 10;
    }

    if unpushed_commits > 0 {
        score -= ((unpushed_commits as i32) / 5) * 5;
    }

    let days_since_modified = (now - last_modified_date).num_days();
    if days_since_modified >= 60 {
        score -= 10;
    }

    if lines_of_code < 100 {
        score -= 5;
    }

    score.clamp(0, 100) as u8
}

pub fn analyze_project(dir: &Path, stale_threshold_days: u32) -> Result<Option<ProjectSnapshot>> {
    let project_type = ProjectType::detect(dir)?;

    let git = match git_health(dir)? {
        Some(g) => g,
        None => return Ok(None),
    };

    let lines_of_code = estimate_loc(dir);

    let last_modified_date = fs::metadata(dir)
        .and_then(|m| m.modified())
        .map(|sys_time| {
            let duration = sys_time
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = duration.as_secs() as i64;
            Utc.timestamp_opt(secs, 0)
                .single()
                .map(|dt| dt.naive_utc())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let health_score = compute_health_score(
        git.is_dirty,
        git.unpushed_commits,
        git.last_commit_date,
        last_modified_date,
        lines_of_code,
        stale_threshold_days,
    );

    Ok(Some(ProjectSnapshot {
        path: dir.to_string_lossy().to_string(),
        project_type: project_type.as_str().to_string(),
        git_branch: git.branch,
        is_dirty: git.is_dirty,
        unpushed_commits: git.unpushed_commits,
        last_commit_date: git.last_commit_date,
        last_modified_date,
        lines_of_code,
        health_score,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_perfect() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(false, 0, now, now, 500, 90);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_health_score_dirty() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(true, 0, now, now, 500, 90);
        assert_eq!(score, 90);
    }

    #[test]
    fn test_health_score_unpushed_commits() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(false, 5, now, now, 500, 90);
        assert_eq!(score, 95);
    }

    #[test]
    fn test_health_score_stale() {
        let now = Utc::now().naive_utc();
        let old = Utc.timestamp_opt(0, 0).single().unwrap().naive_utc();
        let score = compute_health_score(false, 0, old, now, 500, 90);
        assert!(score < 100);
        assert_eq!(score, 85);
    }

    #[test]
    fn test_health_score_floor() {
        let old = Utc.timestamp_opt(0, 0).single().unwrap().naive_utc();
        let score = compute_health_score(true, 200, old, old, 50, 30);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_health_score_small_project() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(false, 0, now, now, 50, 90);
        assert_eq!(score, 95);
    }
}
