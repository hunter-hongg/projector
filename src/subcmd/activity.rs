use std::path::Path;

use anyhow::Result;
use chrono::{TimeZone, Utc};

use crate::analyzer;
use crate::color;
use crate::snapshot::SnapshotStore;

pub fn subcmd_activity(
    days: u32,
    project: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let format = format.unwrap_or_default();
    if !format.is_empty() && format != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", format);
    }

    let projects = if let Some(ref p) = project {
        let dir = Path::new(p);
        if !dir.exists() {
            anyhow::bail!("Path '{}' does not exist", p);
        }
        vec![p.clone()]
    } else {
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
        latest.projects.iter().map(|p| p.path.clone()).collect()
    };

    let mut project_activity: Vec<ProjectActivity> = Vec::new();
    let mut total_commits = 0u32;
    let mut active_count = 0u32;

    for proj_path in &projects {
        let dir = Path::new(proj_path);
        if !dir.exists() {
            continue;
        }

        let project_type = analyzer::ProjectType::detect(dir)
            .ok()
            .map(|t| t.as_str().to_string())
            .unwrap_or_default();

        let stats = match analyzer::count_commits(dir) {
            Ok(Some(s)) => s,
            _ => {
                continue;
            }
        };

        let recent = analyzer::count_commits_since(dir, days)
            .ok()
            .flatten()
            .unwrap_or(0);

        let name = proj_path
            .split('/')
            .next_back()
            .unwrap_or(proj_path);

        let last_commit = get_last_commit_date(dir);

        project_activity.push(ProjectActivity {
            path: proj_path.clone(),
            name: name.to_string(),
            project_type,
            recent_commits: recent,
            total_commits: stats.total,
            authors: stats.authors,
            last_commit_date: last_commit,
        });

        total_commits += recent;
        if recent > 0 {
            active_count += 1;
        }
    }

    if format == "json" {
        let json_projects: Vec<serde_json::Value> = project_activity
            .iter()
            .map(|a| {
                serde_json::json!({
                    "path": a.path,
                    "name": a.name,
                    "type": a.project_type,
                    "recent_commits": a.recent_commits,
                    "total_commits": a.total_commits,
                    "authors": a.authors,
                    "last_commit_date": a.last_commit_date
                })
            })
            .collect();

        let output = serde_json::json!({
            "days": days,
            "total_commits": total_commits,
            "active_projects": active_count,
            "total_projects": project_activity.len(),
            "projects": json_projects
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {}",
            color::info(&format!("Activity — Last {} days", days))
        );
        println!("  ========================================");
        println!();
        println!("  Total commits:      {}", color::cyan(&total_commits.to_string()));
        println!("  Active projects:    {} / {}", color::green(&active_count.to_string()), project_activity.len());
        println!();

        project_activity.sort_by_key(|b| std::cmp::Reverse(b.recent_commits));

        let hot: Vec<_> = project_activity.iter().filter(|a| a.recent_commits > 0).collect();
        let idle: Vec<_> = project_activity.iter().filter(|a| a.recent_commits == 0).collect();

        if !hot.is_empty() {
            println!("  Hottest projects:");
            for a in &hot {
                let type_colored = match a.project_type.as_str() {
                    "Rust" => color::green(&a.project_type),
                    "JavaScript/TypeScript" => color::yellow(&a.project_type),
                    "Go" => color::blue(&a.project_type),
                    "Python" => color::cyan(&a.project_type),
                    _ => color::white(&a.project_type),
                };
                let author_str = if a.authors == 1 {
                    "1 author".to_string()
                } else {
                    format!("{} authors", a.authors)
                };
                println!(
                    "    {:<20} {:<12} {} commits  ({})",
                    color::cyan(&a.name),
                    type_colored,
                    color::green(&a.recent_commits.to_string()),
                    author_str,
                );
            }
        }

        if !idle.is_empty() {
            println!();
            println!("  Idle projects (no activity):");
            for a in &idle {
                let type_colored = match a.project_type.as_str() {
                    "Rust" => color::green(&a.project_type),
                    "JavaScript/TypeScript" => color::yellow(&a.project_type),
                    "Go" => color::blue(&a.project_type),
                    "Python" => color::cyan(&a.project_type),
                    _ => color::white(&a.project_type),
                };
                println!(
                    "    {:<20} {:<12} last commit {}",
                    color::cyan(&a.name),
                    type_colored,
                    color::red(&a.last_commit_date),
                );
            }
        }

        if hot.is_empty() && project.is_some() {
            let days_str = format!("No activity detected in the last {} days.", days);
            println!("  {}", color::yellow(&days_str));
        }
    }

    Ok(())
}

struct ProjectActivity {
    path: String,
    name: String,
    project_type: String,
    recent_commits: u32,
    total_commits: u32,
    authors: u32,
    last_commit_date: String,
}

fn get_last_commit_date(dir: &Path) -> String {
    let repo = match git2::Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return "unknown".to_string(),
    };
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return "no commits".to_string(),
    };
    let commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return "no commits".to_string(),
    };
    let time = commit.time();
    let secs = time.seconds();
    match Utc.timestamp_opt(secs, 0).single() {
        Some(dt) => {
            let now = Utc::now().naive_utc();
            let days = (now - dt.naive_utc()).num_days();
            format!("{} ({}d ago)", dt.format("%Y-%m-%d"), days)
        }
        None => "unknown".to_string(),
    }
}
