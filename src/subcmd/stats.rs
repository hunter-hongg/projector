use anyhow::Result;

use crate::analyzer::{self, ProjectStats};
use crate::color;
use crate::config::Config;
use crate::snapshot::SnapshotStore;

pub fn subcmd_stats(format: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let fmt = format.unwrap_or_default();

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            println!(
                "{}",
                color::error("No snapshot found. Run `projector scan` first.")
            );
            return Ok(());
        }
    };

    let stats = analyzer::compute_stats(&latest, config.report.stale_threshold_days);

    if fmt == "json" {
        print_stats_json(&stats);
    } else {
        print_stats_table(&stats);
    }

    Ok(())
}

fn print_stats_json(stats: &ProjectStats) {
    let json = serde_json::json!({
        "total_projects": stats.total_projects,
        "type_distribution": stats.type_distribution.iter().map(|(t, c, p)| {
            serde_json::json!({"type": t, "count": c, "percent": p})
        }).collect::<Vec<_>>(),
        "avg_health": stats.avg_health,
        "median_health": stats.median_health,
        "std_dev_health": stats.std_dev_health,
        "health_buckets": {
            "high_ge80": stats.health_buckets.0,
            "mid_50_79": stats.health_buckets.1,
            "low_lt50": stats.health_buckets.2,
        },
        "total_loc": stats.total_loc,
        "dirty_ratio": stats.dirty_ratio,
        "stale_ratio": stats.stale_ratio,
        "top5": stats.top5.iter().map(|p| {
            serde_json::json!({
                "path": p.path,
                "health_score": p.health_score,
            })
        }).collect::<Vec<_>>(),
        "bottom5": stats.bottom5.iter().map(|p| {
            serde_json::json!({
                "path": p.path,
                "health_score": p.health_score,
            })
        }).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&json).expect("stats json serialization should not fail"));
}

fn print_stats_table(stats: &ProjectStats) {
    println!();
    println!("  {}", color::info("Global project statistics"));
    println!();
    println!("  Total projects:     {}", color::cyan(&stats.total_projects.to_string()));
    println!("  Total LOC:          {}", stats.total_loc);
    println!("  Average health:     {:.1}/100", stats.avg_health);
    println!("  Median health:      {:.1}/100", stats.median_health);
    println!("  Std deviation:      {:.2}", stats.std_dev_health);
    println!();
    println!("  Health distribution:");
    println!("    ≥ 80 (good):      {} projects", stats.health_buckets.0);
    println!("    50-79 (fair):     {} projects", stats.health_buckets.1);
    println!("    < 50 (poor):      {} projects", stats.health_buckets.2);
    println!();
    println!("  Dirty ratio:        {:.1}%", stats.dirty_ratio * 100.0);
    println!("  Stale ratio:        {:.1}%", stats.stale_ratio * 100.0);
    println!();

    if !stats.type_distribution.is_empty() {
        println!("  Type distribution:");
        for (t, c, p) in &stats.type_distribution {
            println!("    {:30} {:>5} ({:>5.1}%)", t, c, p);
        }
        println!();
    }

    if !stats.top5.is_empty() {
        println!("  Top 5 (highest health):");
        for p in &stats.top5 {
            let name = p.path.split('/').next_back().unwrap_or(&p.path);
            let health_str = format!("{}/100", p.health_score);
            let colored = if p.health_score >= 80 {
                color::green(&health_str)
            } else if p.health_score >= 50 {
                color::yellow(&health_str)
            } else {
                color::red(&health_str)
            };
            println!("    {:30} {}", name, colored);
        }
        println!();
    }

    if !stats.bottom5.is_empty() {
        println!("  Bottom 5 (lowest health):");
        for p in &stats.bottom5 {
            let name = p.path.split('/').next_back().unwrap_or(&p.path);
            let health_str = format!("{}/100", p.health_score);
            let colored = if p.health_score >= 80 {
                color::green(&health_str)
            } else if p.health_score >= 50 {
                color::yellow(&health_str)
            } else {
                color::red(&health_str)
            };
            println!("    {:30} {}", name, colored);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::ProjectSnapshot;

    fn make_scan(projects: Vec<ProjectSnapshot>) -> crate::snapshot::ScanSnapshot {
        crate::snapshot::ScanSnapshot {
            timestamp: chrono::Utc::now().naive_utc(),
            scanned_path: ".".to_string(),
            projects,
        }
    }

    fn make_project(health: u8, loc: u32) -> ProjectSnapshot {
        let now = chrono::Utc::now().naive_utc();
        ProjectSnapshot {
            path: "/test".to_string(),
            project_type: "Rust".to_string(),
            git_branch: "main".to_string(),
            is_dirty: false,
            unpushed_commits: 0,
            last_commit_date: now,
            last_modified_date: now,
            lines_of_code: loc,
            health_score: health,
        }
    }

    #[test]
    fn test_stats_json_output_not_crash() {
        let p = make_project(85, 500);
        let snap = make_scan(vec![p]);
        let stats = analyzer::compute_stats(&snap, 90);
        print_stats_json(&stats);
    }
}
