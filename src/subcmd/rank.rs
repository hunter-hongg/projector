use std::path::Path;

use anyhow::Result;

use crate::analyzer;
use crate::color;
use crate::snapshot::SnapshotStore;

const VALID_METRICS: &[&str] = &["health", "loc", "activity", "age", "commits"];

pub fn subcmd_rank(
    by: Option<String>,
    reverse: bool,
    type_filter: Option<String>,
    top: Option<usize>,
    category: bool,
    format: Option<String>,
) -> Result<()> {
    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    let metric = by.unwrap_or_else(|| "health".to_string());
    if !VALID_METRICS.contains(&metric.as_str()) {
        anyhow::bail!(
            "Invalid metric: '{}'. Valid: {}",
            metric,
            VALID_METRICS.join(", ")
        );
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

    let mut projects: Vec<(String, String, String, String)> = Vec::new();

    for proj in &latest.projects {
        if let Some(ref t) = type_filter
            && !proj.project_type.contains(t)
        {
            continue;
        }

        let name = proj.path.split('/').next_back().unwrap_or(&proj.path);
        let dir = Path::new(&proj.path);

        let value = match metric.as_str() {
            "health" => format!("{}", proj.health_score),
            "loc" => format!("{}", proj.lines_of_code),
            "activity" => {
                let count = if dir.exists() {
                    analyzer::count_commits_since(dir, 30)
                        .ok()
                        .flatten()
                        .unwrap_or(0)
                } else {
                    0
                };
                format!("{}", count)
            }
            "age" => {
                let now = chrono::Utc::now().naive_utc();
                let days = (now - proj.last_commit_date).num_days();
                format!("{}", days)
            }
            "commits" => {
                let count = if dir.exists() {
                    analyzer::count_commits(dir)
                        .ok()
                        .flatten()
                        .map(|s| s.total)
                        .unwrap_or(0)
                } else {
                    0
                };
                format!("{}", count)
            }
            _ => unreachable!(),
        };

        projects.push((
            name.to_string(),
            proj.project_type.clone(),
            value,
            proj.path.clone(),
        ));
    }

    projects.sort_by(|a, b| {
        let a_val: f64 = a.2.parse().unwrap_or(0.0);
        let b_val: f64 = b.2.parse().unwrap_or(0.0);
        if reverse {
            a_val
                .partial_cmp(&b_val)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            b_val
                .partial_cmp(&a_val)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    if let Some(n) = top {
        projects.truncate(n);
    }

    if category {
        let mut by_type: std::collections::BTreeMap<String, Vec<(String, String, String, String)>> =
            std::collections::BTreeMap::new();
        for p in projects {
            by_type.entry(p.1.clone()).or_default().push(p);
        }
        let mut winners: Vec<(String, String, String, String)> = by_type
            .into_values()
            .filter_map(|v| v.into_iter().next())
            .collect();
        winners.sort_by(|a, b| {
            let a_val: f64 = a.2.parse().unwrap_or(0.0);
            let b_val: f64 = b.2.parse().unwrap_or(0.0);
            b_val
                .partial_cmp(&a_val)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        projects = winners;
    }

    if fmt == "json" {
        let json: Vec<serde_json::Value> = projects
            .iter()
            .map(|(name, ptype, val, path)| {
                serde_json::json!({
                    "name": name,
                    "type": ptype,
                    "metric": metric,
                    "value": val,
                    "path": path,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        let metric_upper = match metric.as_str() {
            "health" => "Health".to_string(),
            "loc" => "LOC".to_string(),
            "activity" => "Commits (30d)".to_string(),
            "age" => "Age (days)".to_string(),
            "commits" => "Total Commits".to_string(),
            _ => metric,
        };

        if category {
            println!();
            println!(
                "  {}  {}",
                color::info("▸"),
                color::info(&format!("Per-type leaderboard (by {})", metric_upper))
            );
            println!();
            for (name, ptype, val, _) in &projects {
                let type_colored = match ptype.as_str() {
                    "Rust" => color::green(ptype),
                    "JavaScript/TypeScript" => color::yellow(ptype),
                    "Go" => color::blue(ptype),
                    "Python" => color::cyan(ptype),
                    _ => color::white(ptype),
                };
                println!(
                    "    {:<20}  {:<8}  {:<6}",
                    color::cyan(name),
                    type_colored,
                    color::green(val)
                );
            }
        } else {
            println!();
            println!(
                "  {}  {}",
                color::info("▸"),
                color::info(&format!("Project leaderboard (by {})", metric_upper))
            );
            if reverse {
                println!("  {}", color::info("   (ascending order)"));
            }
            println!();
            let max_name = projects
                .iter()
                .map(|(n, _, _, _)| n.len())
                .max()
                .unwrap_or(20)
                .min(40);
            for (i, (name, ptype, val, _)) in projects.iter().enumerate() {
                let rank = format!("{}.", i + 1);
                let type_colored = match ptype.as_str() {
                    "Rust" => color::green(ptype),
                    "JavaScript/TypeScript" => color::yellow(ptype),
                    "Go" => color::blue(ptype),
                    "Python" => color::cyan(ptype),
                    _ => color::white(ptype),
                };
                let val_colored = if val.parse::<f64>().unwrap_or(0.0) > 0.0 {
                    color::green(val)
                } else {
                    color::red(val)
                };
                println!(
                    "  {:>3} {:<max_name$}  {:<8}  {}",
                    color::cyan(&rank),
                    color::blue(name),
                    type_colored,
                    val_colored,
                    max_name = max_name,
                );
            }
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_invalid_format() {
        let result = subcmd_rank(None, false, None, None, false, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported format")
        );
    }

    #[test]
    fn test_rank_invalid_by() {
        let result = subcmd_rank(
            Some("invalid_metric".to_string()),
            false,
            None,
            None,
            false,
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }

    #[test]
    fn test_rank_graceful_no_snapshot() {
        let result = subcmd_rank(None, false, None, None, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_metrics_list() {
        assert!(VALID_METRICS.contains(&"health"));
        assert!(VALID_METRICS.contains(&"loc"));
        assert!(VALID_METRICS.contains(&"activity"));
        assert!(VALID_METRICS.contains(&"age"));
        assert!(VALID_METRICS.contains(&"commits"));
        assert_eq!(VALID_METRICS.len(), 5);
    }
}
