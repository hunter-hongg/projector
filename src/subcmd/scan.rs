use std::path::Path;

use anyhow::Result;
use chrono::Utc;

use crate::analyzer;
use crate::color;
use crate::config::Config;
use crate::snapshot::{format_health_deductions, ScanSnapshot, SnapshotStore};

pub fn subcmd_scan(dir: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let dir = dir.unwrap_or(config.scan.default_path.clone());
    let dir_path = Path::new(&dir);

    if !dir_path.is_dir() {
        anyhow::bail!("Directory not found: {}", dir);
    }

    let stale_threshold = config.report.stale_threshold_days;
    let mut projects = Vec::new();

    let (project_dirs, _) = analyzer::classify_dirs(dir_path, true)?;
    for path in project_dirs {
        if let Some(snapshot) = analyzer::analyze_project(&path, stale_threshold)? {
            projects.push(snapshot);
        }
    }

    let snapshot = ScanSnapshot {
        timestamp: Utc::now().naive_utc(),
        scanned_path: dir,
        projects,
    };

    SnapshotStore::save(&snapshot)?;
    println!(
        "Scanned {} projects, snapshot saved.",
        snapshot.projects.len()
    );

    let alert_threshold = config.alert.health_threshold;
    if alert_threshold > 0 {
        let mut low_health: Vec<_> = snapshot
            .projects
            .iter()
            .filter(|p| u32::from(p.health_score) < alert_threshold)
            .collect();
        if !low_health.is_empty() {
            low_health.sort_by_key(|p| p.health_score);
            println!();
            println!(
                "  {}",
            color::yellow(&format!(
                "Projects with health score below {}: ",
                alert_threshold
            ))
            );
            for p in &low_health {
                let name = p.path.split('/').next_back().unwrap_or(&p.path);
                let deductions = format_health_deductions(
                    p.is_dirty,
                    p.last_commit_date,
                    p.lines_of_code,
                    stale_threshold,
                );
                let reason = if deductions.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", deductions.join(", "))
                };
                println!(
                    "  - {:<20} {:>2}/100{}",
                    color::red(name),
                    p.health_score,
                    reason,
                );
            }
            println!();
        }
    }

    Ok(())
}
