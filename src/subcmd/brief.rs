use std::path::Path;

use anyhow::Result;
use chrono::Utc;

use crate::color;
use crate::config::Config;
use crate::git;
use crate::snapshot::SnapshotStore;

pub fn subcmd_brief(days: u32, format: Option<String>) -> Result<()> {
    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    let config = Config::load()?;
    let stale_threshold = config.report.stale_threshold_days;

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

    let total = latest.projects.len();
    let total_loc: u32 = latest.projects.iter().map(|p| p.lines_of_code).sum();

    let high = latest
        .projects
        .iter()
        .filter(|p| p.health_score >= 80)
        .count();
    let mid = latest
        .projects
        .iter()
        .filter(|p| p.health_score >= 50 && p.health_score < 80)
        .count();
    let low = latest
        .projects
        .iter()
        .filter(|p| p.health_score < 50)
        .count();

    let avg_health = if total > 0 {
        latest
            .projects
            .iter()
            .map(|p| p.health_score as f64)
            .sum::<f64>()
            / total as f64
    } else {
        0.0
    };

    let dirty = latest.projects.iter().filter(|p| p.is_dirty).count();

    let now = Utc::now().naive_utc();
    let stale = latest
        .projects
        .iter()
        .filter(|p| {
            let days_since = (now - p.last_commit_date).num_days();
            days_since >= stale_threshold as i64
        })
        .count();

    let mut active: Vec<(String, String, u32)> = Vec::new();
    for proj in &latest.projects {
        let dir = Path::new(&proj.path);
        if !dir.exists() {
            continue;
        }
        if let Ok(Some(count)) = git::count_commits_since(dir, days)
            && count > 0
        {
            let name = proj
                .path
                .split('/')
                .next_back()
                .unwrap_or(&proj.path)
                .to_string();
            active.push((name, proj.project_type.clone(), count));
        }
    }
    active.sort_by_key(|(_, _, c)| std::cmp::Reverse(*c));

    if fmt == "json" {
        let json = serde_json::json!({
            "date": now.format("%Y-%m-%d").to_string(),
            "total_projects": total,
            "avg_health": format!("{:.1}", avg_health),
            "health_buckets": {
                "good_ge80": high,
                "fair_50_79": mid,
                "poor_lt50": low,
            },
            "total_loc": total_loc,
            "dirty": dirty,
            "stale": stale,
            "active_projects": active.len(),
            "total_commits": active.iter().map(|(_, _, c)| c).sum::<u32>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!();
        println!(
            "  {}",
            color::info(&format!("Project Brief — {}", now.format("%Y-%m-%d")))
        );
        println!("  {}", "═".repeat(40));
        println!();
        println!("  Total:  {} projects", color::cyan(&total.to_string()));
        println!(
            "  Health: {:.1} avg · {} good · {} fair · {} poor",
            avg_health, high, mid, low
        );
        println!("  LOC:    {}", total_loc);
        println!("  Dirty:  {} projects", dirty);
        println!("  Stale:  {} projects (>{}d)", stale, stale_threshold);
        println!();

        if !active.is_empty() {
            println!("  {}  Activity (last {}d):", color::info("▸"), days);
            for (name, ptype, count) in &active {
                let type_colored = match ptype.as_str() {
                    "Rust" => color::green("Rust"),
                    "JavaScript/TypeScript" => color::yellow("JS"),
                    "Go" => color::blue("Go"),
                    "Python" => color::cyan("Py"),
                    _ => color::white(ptype),
                };
                println!(
                    "    {:<20} {:<6} {} commit{}",
                    color::cyan(name),
                    type_colored,
                    count,
                    if *count == 1 { "" } else { "s" }
                );
            }
        } else {
            println!("  No activity in the last {} days.", days);
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brief_invalid_format() {
        let result = subcmd_brief(1, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported format")
        );
    }

    #[test]
    fn test_brief_graceful_no_snapshot() {
        let result = subcmd_brief(1, None);
        assert!(result.is_ok());
    }
}
