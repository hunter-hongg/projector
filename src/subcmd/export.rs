use anyhow::Result;

use crate::analyzer;
use crate::config::Config;
use crate::export_template::{self, DashboardData, DashboardProject, RankItem, TypeDistItem};
use crate::snapshot::SnapshotStore;

fn ensure_parent_dir(path: &str) -> Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn build_dashboard_data(
    latest: &crate::snapshot::ScanSnapshot,
    config: &Config,
) -> DashboardData {
    let stats = analyzer::compute_stats(latest, config.report.stale_threshold_days);

    let type_dist: Vec<TypeDistItem> = stats
        .type_distribution
        .iter()
        .map(|(name, count, _)| TypeDistItem {
            name: name.clone(),
            count: *count,
        })
        .collect();

    let top5: Vec<RankItem> = stats
        .top5
        .iter()
        .map(|p| RankItem {
            name: p.path.split('/').next_back().unwrap_or(&p.path).to_string(),
            health: p.health_score,
        })
        .collect();

    let bottom5: Vec<RankItem> = stats
        .bottom5
        .iter()
        .map(|p| RankItem {
            name: p.path.split('/').next_back().unwrap_or(&p.path).to_string(),
            health: p.health_score,
        })
        .collect();

    let projects: Vec<DashboardProject> = {
        let mut sorted = latest.projects.clone();
        sorted.sort_by_key(|p| std::cmp::Reverse(p.health_score));
        sorted
            .iter()
            .map(|p| {
                let status = if p.is_dirty {
                    "dirty".to_string()
                } else {
                    use chrono::Utc;
                    let now = Utc::now().naive_utc();
                    let days = (now - p.last_commit_date).num_days();
                    if days >= config.report.stale_threshold_days as i64 {
                        "stale".to_string()
                    } else {
                        "clean".to_string()
                    }
                };
                DashboardProject {
                    name: p.path.split('/').next_back().unwrap_or(&p.path).to_string(),
                    project_type: p.project_type.clone(),
                    branch: p.git_branch.clone(),
                    status,
                    health: p.health_score,
                }
            })
            .collect()
    };

    DashboardData {
        project_count: stats.total_projects,
        avg_health: stats.avg_health,
        total_loc: stats.total_loc,
        dirty_ratio: stats.dirty_ratio,
        stale_ratio: stats.stale_ratio,
        health_high: stats.health_buckets.0,
        health_mid: stats.health_buckets.1,
        health_low: stats.health_buckets.2,
        projects,
        type_distribution: type_dist,
        top5,
        bottom5,
        has_data: stats.total_projects > 0,
        scanned_at: latest.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
    }
}

pub fn subcmd_export_html(output: Option<String>) -> Result<()> {
    let config = Config::load()?;

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            let html = export_template::render_empty_html();
            if let Some(ref path) = output {
                ensure_parent_dir(path)?;
                std::fs::write(path, html)?;
                println!("Exported empty dashboard to {}", path);
            } else {
                println!("{}", html);
            }
            return Ok(());
        }
    };

    let data = build_dashboard_data(&latest, &config);
    let html = export_template::render_html(&data);

    if let Some(ref path) = output {
        ensure_parent_dir(path)?;
        std::fs::write(path, html)?;
        println!("Exported dashboard to {}", path);
    } else {
        println!("{}", html);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_data_serialize() {
        let data = DashboardData {
            project_count: 0,
            avg_health: 0.0,
            total_loc: 0,
            dirty_ratio: 0.0,
            stale_ratio: 0.0,
            health_high: 0,
            health_mid: 0,
            health_low: 0,
            projects: vec![],
            type_distribution: vec![],
            top5: vec![],
            bottom5: vec![],
            has_data: false,
            scanned_at: "2026-01-01 00:00:00".to_string(),
        };
        let html = export_template::render_html(&data);
        assert!(html.contains("Projector Dashboard"));
        assert!(html.contains("0 projects"));
    }

    #[test]
    fn test_render_empty() {
        let html = export_template::render_empty_html();
        assert!(html.contains("No snapshot found"));
    }

    #[test]
    fn test_dashboard_with_projects() {
        let data = DashboardData {
            project_count: 2,
            avg_health: 75.0,
            total_loc: 1000,
            dirty_ratio: 0.5,
            stale_ratio: 0.0,
            health_high: 1,
            health_mid: 1,
            health_low: 0,
            projects: vec![
                DashboardProject {
                    name: "proj-a".to_string(),
                    project_type: "Rust".to_string(),
                    branch: "main".to_string(),
                    status: "clean".to_string(),
                    health: 90,
                },
                DashboardProject {
                    name: "proj-b".to_string(),
                    project_type: "Python".to_string(),
                    branch: "dev".to_string(),
                    status: "dirty".to_string(),
                    health: 60,
                },
            ],
            type_distribution: vec![
                TypeDistItem {
                    name: "Rust".to_string(),
                    count: 1,
                },
                TypeDistItem {
                    name: "Python".to_string(),
                    count: 1,
                },
            ],
            top5: vec![RankItem {
                name: "proj-a".to_string(),
                health: 90,
            }],
            bottom5: vec![RankItem {
                name: "proj-b".to_string(),
                health: 60,
            }],
            has_data: true,
            scanned_at: "2026-05-22 12:00:00".to_string(),
        };
        let html = export_template::render_html(&data);
        assert!(html.contains("proj-a"));
        assert!(html.contains("proj-b"));
        assert!(html.contains("90"));
        assert!(html.contains("60"));
    }
}
