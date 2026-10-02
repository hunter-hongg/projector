use std::collections::HashMap;

use crate::snapshot::ProjectSnapshot;

pub struct ProjectStats {
    pub total_projects: usize,
    pub type_distribution: Vec<(String, usize, f64)>,
    pub avg_health: f64,
    pub median_health: f64,
    pub std_dev_health: f64,
    pub health_buckets: (usize, usize, usize),
    pub top5: Vec<ProjectSnapshot>,
    pub bottom5: Vec<ProjectSnapshot>,
    pub total_loc: u32,
    pub dirty_ratio: f64,
    pub stale_ratio: f64,
}

pub fn compute_stats(
    snapshot: &crate::snapshot::ScanSnapshot,
    stale_threshold_days: u32,
) -> ProjectStats {
    use chrono::Utc;

    let now = Utc::now().naive_utc();
    let total = snapshot.projects.len();

    let mut type_map: HashMap<String, usize> = HashMap::new();
    for p in &snapshot.projects {
        *type_map.entry(p.project_type.clone()).or_insert(0) += 1;
    }
    let type_distribution: Vec<(String, usize, f64)> = type_map
        .into_iter()
        .map(|(k, v)| {
            let pct = if total > 0 {
                (v as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            (k, v, pct)
        })
        .collect();

    let mut sorted: Vec<&ProjectSnapshot> = snapshot.projects.iter().collect();
    sorted.sort_by_key(|p| p.health_score);

    let avg_health = if total > 0 {
        sorted.iter().map(|p| p.health_score as f64).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let median_health = if total > 0 {
        let mid = total / 2;
        if total.is_multiple_of(2) {
            (sorted[mid - 1].health_score as f64 + sorted[mid].health_score as f64) / 2.0
        } else {
            sorted[mid].health_score as f64
        }
    } else {
        0.0
    };

    let std_dev_health = if total > 0 {
        let variance = sorted
            .iter()
            .map(|p| {
                let diff = p.health_score as f64 - avg_health;
                diff * diff
            })
            .sum::<f64>()
            / total as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let high = sorted.iter().filter(|p| p.health_score >= 80).count();
    let mid = sorted
        .iter()
        .filter(|p| p.health_score >= 50 && p.health_score < 80)
        .count();
    let low = sorted.iter().filter(|p| p.health_score < 50).count();

    let top5: Vec<ProjectSnapshot> = sorted.iter().rev().take(5).map(|p| (*p).clone()).collect();
    let bottom5: Vec<ProjectSnapshot> = sorted.iter().take(5).map(|p| (*p).clone()).collect();

    let total_loc: u32 = snapshot.projects.iter().map(|p| p.lines_of_code).sum();
    let dirty_count = snapshot.projects.iter().filter(|p| p.is_dirty).count();
    let dirty_ratio = if total > 0 {
        dirty_count as f64 / total as f64
    } else {
        0.0
    };

    let stale_count = snapshot
        .projects
        .iter()
        .filter(|p| {
            let days = (now - p.last_commit_date).num_days();
            days >= stale_threshold_days as i64
        })
        .count();
    let stale_ratio = if total > 0 {
        stale_count as f64 / total as f64
    } else {
        0.0
    };

    ProjectStats {
        total_projects: total,
        type_distribution,
        avg_health,
        median_health,
        std_dev_health,
        health_buckets: (high, mid, low),
        top5,
        bottom5,
        total_loc,
        dirty_ratio,
        stale_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::ScanSnapshot;
    use chrono::Utc;

    #[test]
    fn test_compute_stats_empty() {
        let snapshot = ScanSnapshot {
            timestamp: Utc::now().naive_utc(),
            scanned_path: ".".to_string(),
            projects: vec![],
        };
        let stats = compute_stats(&snapshot, 90);
        assert_eq!(stats.total_projects, 0);
        assert_eq!(stats.avg_health, 0.0);
        assert_eq!(stats.median_health, 0.0);
        assert_eq!(stats.std_dev_health, 0.0);
        assert_eq!(stats.total_loc, 0);
    }

    #[test]
    fn test_compute_stats_single_project() {
        let now = Utc::now().naive_utc();
        let p = ProjectSnapshot {
            path: "/test".to_string(),
            project_type: "Rust".to_string(),
            git_branch: "main".to_string(),
            is_dirty: false,
            unpushed_commits: 0,
            last_commit_date: now,
            last_modified_date: now,
            lines_of_code: 500,
            health_score: 85,
        };
        let snapshot = ScanSnapshot {
            timestamp: now,
            scanned_path: ".".to_string(),
            projects: vec![p],
        };
        let stats = compute_stats(&snapshot, 90);
        assert_eq!(stats.total_projects, 1);
        assert!((stats.avg_health - 85.0).abs() < 0.001);
        assert!((stats.median_health - 85.0).abs() < 0.001);
        assert!((stats.std_dev_health - 0.0).abs() < 0.001);
        assert_eq!(stats.total_loc, 500);
    }

    #[test]
    fn test_compute_stats_multiple_projects() {
        let now = Utc::now().naive_utc();
        let projects = vec![
            ProjectSnapshot {
                path: "/a".to_string(),
                project_type: "Rust".to_string(),
                git_branch: "main".to_string(),
                is_dirty: false,
                unpushed_commits: 0,
                last_commit_date: now,
                last_modified_date: now,
                lines_of_code: 100,
                health_score: 100,
            },
            ProjectSnapshot {
                path: "/b".to_string(),
                project_type: "Python".to_string(),
                git_branch: "main".to_string(),
                is_dirty: true,
                unpushed_commits: 0,
                last_commit_date: now,
                last_modified_date: now,
                lines_of_code: 200,
                health_score: 80,
            },
            ProjectSnapshot {
                path: "/c".to_string(),
                project_type: "Rust".to_string(),
                git_branch: "main".to_string(),
                is_dirty: false,
                unpushed_commits: 0,
                last_commit_date: now,
                last_modified_date: now,
                lines_of_code: 300,
                health_score: 60,
            },
        ];
        let snapshot = ScanSnapshot {
            timestamp: now,
            scanned_path: ".".to_string(),
            projects,
        };
        let stats = compute_stats(&snapshot, 90);
        assert_eq!(stats.total_projects, 3);
        assert!((stats.avg_health - 80.0).abs() < 0.001);
        assert!((stats.median_health - 80.0).abs() < 0.001);
        assert_eq!(stats.total_loc, 600);
        assert!((stats.dirty_ratio - 1.0 / 3.0).abs() < 0.001);
    }
}
