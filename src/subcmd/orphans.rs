use std::path::Path;

use anyhow::Result;

use crate::color;
use crate::snapshot::{ProjectSnapshot, SnapshotStore};

pub fn subcmd_orphans(
    days: u32,
    all: bool,
    format: Option<String>,
) -> Result<()> {
    let format = format.unwrap_or_default();
    if !format.is_empty() && format != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", format);
    }

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            println!(
                "{}",
                color::error("No snapshots found. Run `projector scan` first.")
            );
            return Ok(());
        }
    };

    let now = chrono::Utc::now().naive_utc();

    let mut orphan_projects: Vec<&ProjectSnapshot> = Vec::new();
    let mut non_orphan_projects: Vec<&ProjectSnapshot> = Vec::new();
    let mut no_git_projects: Vec<&ProjectSnapshot> = Vec::new();

    for proj in &latest.projects {
        let dir = Path::new(&proj.path);

        if !dir.join(".git").exists() {
            no_git_projects.push(proj);
            continue;
        }

        let has_remote = has_origin_remote(dir);
        let days_since_commit = (now - proj.last_commit_date).num_days();

        if !has_remote && days_since_commit > days as i64 {
            orphan_projects.push(proj);
        } else {
            non_orphan_projects.push(proj);
        }
    }

    if format == "json" {
        let json_orphans: Vec<serde_json::Value> = orphan_projects
            .iter()
            .map(|p| {
                let days_since = (now - p.last_commit_date).num_days();
                serde_json::json!({
                    "path": p.path,
                    "name": p.path.split('/').next_back().unwrap_or(&p.path),
                    "type": p.project_type,
                    "last_commit": p.last_commit_date.format("%Y-%m-%d").to_string(),
                    "days_since_commit": days_since
                })
            })
            .collect();

        let output = if all {
            let json_non: Vec<serde_json::Value> = non_orphan_projects
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "path": p.path,
                        "name": p.path.split('/').next_back().unwrap_or(&p.path),
                        "type": p.project_type,
                        "is_orphan": false
                    })
                })
                .collect();
            let json_no_git: Vec<serde_json::Value> = no_git_projects
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "path": p.path,
                        "name": p.path.split('/').next_back().unwrap_or(&p.path),
                        "type": p.project_type,
                        "is_orphan": false,
                        "no_git": true
                    })
                })
                .collect();
            serde_json::json!({
                "orphans": json_orphans,
                "non_orphans": json_non,
                "no_git": json_no_git,
                "total_orphans": orphan_projects.len(),
                "total_projects": latest.projects.len()
            })
        } else {
            serde_json::json!({
                "orphans": json_orphans,
                "total_orphans": orphan_projects.len(),
                "total_projects": latest.projects.len()
            })
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {}",
            color::info(&format!(
                "Orphan Projects (no remote + no activity >{}d)",
                days
            ))
        );
        println!("  ================================================");
        println!();

        if orphan_projects.is_empty() {
            println!("  {}  No orphan projects!", color::green("🎉"));
            if !all {
                return Ok(());
            }
        }

        for p in &orphan_projects {
            let name = p.path.split('/').next_back().unwrap_or(&p.path);
            let days_since = (now - p.last_commit_date).num_days();
            let type_colored = match p.project_type.as_str() {
                "Rust" => color::green(&p.project_type),
                "JavaScript/TypeScript" => color::yellow(&p.project_type),
                "Go" => color::blue(&p.project_type),
                "Python" => color::cyan(&p.project_type),
                _ => color::white(&p.project_type),
            };
            println!(
                "    {:<20} {:<12} last commit: {} ({}d ago)",
                color::cyan(name),
                type_colored,
                color::red(&p.last_commit_date.format("%Y-%m-%d").to_string()),
                days_since,
            );
        }

        if all {
            for p in &no_git_projects {
                let name = p.path.split('/').next_back().unwrap_or(&p.path);
                println!(
                    "    {:<20} {:<12} No git",
                    color::cyan(name),
                    color::white(&p.project_type),
                );
            }
        }

        println!();
        println!(
            "  Total: {} orphan projects out of {} scanned",
            color::yellow(&orphan_projects.len().to_string()),
            latest.projects.len()
        );
    }

    Ok(())
}

fn has_origin_remote(dir: &Path) -> bool {
    let repo = match git2::Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return false,
    };
    repo.find_remote("origin").is_ok()
}
