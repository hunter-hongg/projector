use std::path::Path;

use anyhow::Result;
use chrono::Utc;

use crate::analyzer;
use crate::config::Config;
use crate::snapshot::{ScanSnapshot, SnapshotStore};

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
    Ok(())
}
