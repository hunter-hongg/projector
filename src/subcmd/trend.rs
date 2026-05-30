use anyhow::Result;

use crate::analyzer::{self, TrendPoint};
use crate::color;
use crate::snapshot::SnapshotStore;

pub fn subcmd_trend(
    path: Option<String>,
    days: Option<u32>,
    metric: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let fmt = format.unwrap_or_default();
    let metric = metric.unwrap_or_else(|| "health".to_string());

    let snapshots = SnapshotStore::load_all()?;

    if snapshots.len() < 2 {
        println!(
            "{}",
            color::info("Need at least 2 snapshots for trend (found {})",)
        );
        return Ok(());
    }

    let filtered: Vec<_> = if let Some(d) = days {
        let cutoff = chrono::Utc::now().naive_utc()
            - chrono::Duration::days(d as i64);
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
                let proj = s.projects.iter().find(|proj| {
                    proj.path == *p || proj.path.ends_with(p.as_str())
                })?;
                let val = match metric.as_str() {
                    "loc" => proj.lines_of_code as f64,
                    _ => proj.health_score as f64,
                };
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
                let val = if metric == "loc" {
                    s.projects.iter().map(|p| p.lines_of_code as f64).sum::<f64>()
                } else {
                    s.projects.iter().map(|p| p.health_score as f64).sum::<f64>()
                        / s.projects.len() as f64
                };
                TrendPoint {
                    date: s.timestamp.format("%Y-%m-%d").to_string(),
                    value: val,
                }
            })
            .collect()
    };

    if fmt == "json" {
        let json: Vec<_> = points
            .iter()
            .map(|p| {
                serde_json::json!({
                    "date": p.date,
                    "value": p.value,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    let metric_label = if metric == "loc" { "LOC" } else { "Health Score" };

    println!();
    println!(
        "  {}",
        color::info(&format!("{} Trend", metric_label))
    );
    if let Some(ref p) = path {
        println!("  Project: {}", color::cyan(p));
    }
    println!();

    let chart = analyzer::draw_ascii_chart(&points, 60, 12);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_no_snapshots() {
        let result = subcmd_trend(None, None, None, None);
        assert!(result.is_ok());
    }
}
