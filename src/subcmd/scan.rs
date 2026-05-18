use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::Utc;

use crate::analyzer::analyze_project;
use crate::snapshot::{ScanSnapshot, SnapshotStore};
use crate::config::Config;

pub fn subcmd_scan(dir: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let dir = dir.unwrap_or(config.scan.default_path.clone());
    let dir_path = Path::new(&dir);

    if !dir_path.is_dir() {
        anyhow::bail!("Directory not found: {}", dir);
    }

    let stale_threshold = config.report.stale_threshold_days;
    let mut projects = Vec::new();

    let entries = fs::read_dir(dir_path)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }

        if let Some(snapshot) = analyze_project(&path, stale_threshold)? {
            projects.push(snapshot);
        }
    }

    let snapshot = ScanSnapshot {
        timestamp: Utc::now().naive_utc(),
        scanned_path: dir,
        projects,
    };

    SnapshotStore::save(&snapshot)?;
    println!("Scanned {} projects, snapshot saved.", snapshot.projects.len());
    Ok(())
}
