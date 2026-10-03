use anyhow::Result;

use crate::chart::{self, TrendPoint};
use crate::color;
use crate::format::OutputFormat;
use crate::snapshot::{ScanSnapshot, SnapshotStore};

/// Metrics `trend --metric` accepts, in the order the `--help` lists them.
pub const VALID_METRICS: &[&str] = &["health", "loc", "unpushed", "dirty", "age", "projects"];

pub fn subcmd_trend(
    path: Option<String>,
    days: Option<u32>,
    metric: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let fmt = OutputFormat::parse(
        format.as_deref(),
        &[OutputFormat::Json, OutputFormat::Markdown],
    )?;
    let metric = metric.unwrap_or_else(|| "health".to_string());
    if !VALID_METRICS.contains(&metric.as_str()) {
        anyhow::bail!(
            "Invalid metric: '{}'. Valid: {}",
            metric,
            VALID_METRICS.join(", ")
        );
    }

    let snapshots = SnapshotStore::load_all()?;

    if snapshots.len() < 2 {
        println!(
            "{}",
            color::info("Need at least 2 snapshots for trend (found {})",)
        );
        return Ok(());
    }

    let filtered: Vec<_> = if let Some(d) = days {
        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(d as i64);
        snapshots
            .into_iter()
            .filter(|s| s.timestamp >= cutoff)
            .collect()
    } else {
        snapshots
    };

    if filtered.len() < 2 {
        println!(
            "{}",
            color::info("Not enough snapshots in the specified time range")
        );
        return Ok(());
    }

    let points: Vec<TrendPoint> = if let Some(ref p) = path {
        filtered
            .iter()
            .filter_map(|s| {
                let proj = s
                    .projects
                    .iter()
                    .find(|proj| proj.path == *p || proj.path.ends_with(p.as_str()))?;
                let val = project_metric(proj, &metric);
                Some(TrendPoint {
                    date: s.timestamp.format("%Y-%m-%d").to_string(),
                    value: val,
                })
            })
            .collect()
    } else {
        filtered
            .iter()
            .map(|s| {
                let val = snapshot_metric(s, &metric);
                TrendPoint {
                    date: s.timestamp.format("%Y-%m-%d").to_string(),
                    value: val,
                }
            })
            .collect()
    };

    if fmt.is_json() {
        let json: Vec<_> = points
            .iter()
            .map(|p| {
                serde_json::json!({
                    "date": p.date,
                    "value": p.value,
                    "metric": metric,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    if fmt.is_markdown() {
        println!("# {} Trend", metric_label(&metric));
        if let Some(ref p) = path {
            println!();
            println!("Project: `{}`", p);
        }
        println!();
        println!("| Date | {} |", metric_label(&metric));
        println!("|------|------|");
        for p in &points {
            println!("| {} | {:.1} |", p.date, p.value);
        }
        return Ok(());
    }

    println!();
    println!(
        "  {}",
        color::info(&format!("{} Trend", metric_label(&metric)))
    );
    if let Some(ref p) = path {
        println!("  Project: {}", color::cyan(p));
    }
    println!();

    let chart = chart::draw_ascii_chart(&points, 60, 12);
    for line in &chart {
        println!("  {}", line);
    }
    println!();

    println!("  Data points:");
    for p in &points {
        println!("    {:12}  {:.1}", p.date, p.value);
    }

    Ok(())
}

/// Metric value for a single project in one snapshot.
fn project_metric(proj: &crate::snapshot::ProjectSnapshot, metric: &str) -> f64 {
    match metric {
        "loc" => proj.lines_of_code as f64,
        "unpushed" => proj.unpushed_commits as f64,
        "dirty" => u8::from(proj.is_dirty) as f64,
        "age" => {
            let now = chrono::Utc::now().naive_utc();
            (now - proj.last_commit_date).num_days() as f64
        }
        "projects" => 1.0,
        // health
        _ => proj.health_score as f64,
    }
}

/// Aggregate metric across every project in one snapshot.
fn snapshot_metric(s: &ScanSnapshot, metric: &str) -> f64 {
    if s.projects.is_empty() {
        return 0.0;
    }
    match metric {
        "loc" => s.projects.iter().map(|p| p.lines_of_code as f64).sum(),
        "unpushed" => s.projects.iter().map(|p| p.unpushed_commits as f64).sum(),
        "dirty" => {
            let dirty = s.projects.iter().filter(|p| p.is_dirty).count() as f64;
            dirty / s.projects.len() as f64
        }
        "age" => {
            let now = chrono::Utc::now().naive_utc();
            s.projects
                .iter()
                .map(|p| (now - p.last_commit_date).num_days() as f64)
                .sum::<f64>()
                / s.projects.len() as f64
        }
        "projects" => s.projects.len() as f64,
        // health
        _ => {
            s.projects
                .iter()
                .map(|p| p.health_score as f64)
                .sum::<f64>()
                / s.projects.len() as f64
        }
    }
}

/// Human-readable label for the CLI header and Markdown table title.
fn metric_label(metric: &str) -> String {
    match metric {
        "loc" => "LOC".to_string(),
        "unpushed" => "Unpushed Commits".to_string(),
        "dirty" => "Dirty Ratio".to_string(),
        "age" => "Avg Age (days)".to_string(),
        "projects" => "Projects".to_string(),
        // health
        _ => "Health Score".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::ProjectSnapshot;
    use chrono::Utc;

    fn mk_project(
        path: &str,
        health: u8,
        loc: u32,
        dirty: bool,
        unpushed: u32,
        last_commit: chrono::NaiveDateTime,
    ) -> ProjectSnapshot {
        ProjectSnapshot {
            path: path.to_string(),
            project_type: "Rust".to_string(),
            git_branch: "main".to_string(),
            is_dirty: dirty,
            unpushed_commits: unpushed,
            last_commit_date: last_commit,
            last_modified_date: last_commit,
            lines_of_code: loc,
            health_score: health,
            depth: 1,
        }
    }

    fn mk_scan(projects: Vec<ProjectSnapshot>, timestamp: chrono::NaiveDateTime) -> ScanSnapshot {
        ScanSnapshot {
            timestamp,
            scanned_path: ".".to_string(),
            projects,
            schema_version: crate::snapshot::SCHEMA_VERSION,
        }
    }

    fn now() -> chrono::NaiveDateTime {
        Utc::now().naive_utc()
    }

    #[test]
    fn test_snapshot_metric_aggregates() {
        let old = now() - chrono::Duration::days(100);
        let s = mk_scan(
            vec![
                mk_project("a", 80, 300, true, 2, now()),
                mk_project("b", 60, 100, false, 0, old),
            ],
            now(),
        );

        assert_eq!(snapshot_metric(&s, "health"), 70.0);
        assert_eq!(snapshot_metric(&s, "loc"), 400.0);
        assert_eq!(snapshot_metric(&s, "unpushed"), 2.0);
        assert_eq!(snapshot_metric(&s, "dirty"), 0.5);
        assert_eq!(snapshot_metric(&s, "projects"), 2.0);
        // avg age = (0 + 100) / 2
        assert_eq!(snapshot_metric(&s, "age"), 50.0);
    }

    #[test]
    fn test_snapshot_metric_empty_projects_is_zero() {
        let s = mk_scan(vec![], now());
        assert_eq!(snapshot_metric(&s, "health"), 0.0);
        assert_eq!(snapshot_metric(&s, "dirty"), 0.0);
        assert_eq!(snapshot_metric(&s, "projects"), 0.0);
    }

    #[test]
    fn test_project_metric_per_project() {
        let old = now() - chrono::Duration::days(30);
        let p = mk_project("a", 55, 250, true, 7, old);
        assert_eq!(project_metric(&p, "health"), 55.0);
        assert_eq!(project_metric(&p, "loc"), 250.0);
        assert_eq!(project_metric(&p, "unpushed"), 7.0);
        assert_eq!(project_metric(&p, "dirty"), 1.0);
        assert_eq!(project_metric(&p, "age"), 30.0);
        assert_eq!(project_metric(&p, "projects"), 1.0);
    }

    #[test]
    fn test_metric_labels() {
        assert_eq!(metric_label("health"), "Health Score");
        assert_eq!(metric_label("loc"), "LOC");
        assert_eq!(metric_label("unpushed"), "Unpushed Commits");
        assert_eq!(metric_label("dirty"), "Dirty Ratio");
        assert_eq!(metric_label("age"), "Avg Age (days)");
        assert_eq!(metric_label("projects"), "Projects");
    }

    #[test]
    fn test_trend_no_snapshots() {
        let result = subcmd_trend(None, None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trend_invalid_metric_is_rejected() {
        let result = subcmd_trend(None, None, Some("bogus".to_string()), None);
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Invalid metric"), "{msg}");
        assert!(msg.contains("bogus"), "{msg}");
    }

    #[test]
    fn test_trend_markdown_format_is_accepted() {
        let parsed =
            OutputFormat::parse(Some("md"), &[OutputFormat::Json, OutputFormat::Markdown]).unwrap();
        assert!(parsed.is_markdown());
    }
}
