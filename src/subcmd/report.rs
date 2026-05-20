use anyhow::Result;

use crate::color;
use crate::config::Config;
use crate::snapshot::SnapshotStore;

pub fn subcmd_report(diff: bool, format: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let stale_threshold = config.report.stale_threshold_days;
    let format = format.unwrap_or_default();
    if !format.is_empty() && format != "json" && format != "md" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json' or 'md'.", format);
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

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&latest)?);
        return Ok(());
    }

    if format == "md" {
        println!("# Project Health Report");
        println!();
        println!(
            "Scanned: {} | Path: {}",
            latest.timestamp.format("%Y-%m-%d %H:%M:%S"),
            latest.scanned_path
        );
        println!();
        println!("| Project | Type | Branch | Status | Last Commit | Health |");
        println!("|---------|------|--------|--------|-------------|--------|");
        for p in &latest.projects {
            let status = status_label(p.is_dirty, p.last_commit_date, stale_threshold);
            let health = format!("{}/100", p.health_score);
            println!(
                "| {} | {} | {} | {} | {} | {} |",
                p.path.split('/').next_back().unwrap_or(&p.path),
                p.project_type,
                p.git_branch,
                status,
                p.last_commit_date.format("%Y-%m-%d"),
                health,
            );
        }
        return Ok(());
    }

    println!();
    println!(
        "  {}",
        color::info(&format!(
            "Project Health Dashboard — {} — {}",
            latest.timestamp.format("%Y-%m-%d %H:%M:%S"),
            latest.scanned_path
        ))
    );
    println!();

    const W: [usize; 6] = [28, 22, 12, 8, 12, 8];

    println!("{}", table_top(&W));
    println!(
        "{}",
        table_row(
            &W,
            &[
                color::cyan("Project"),
                color::cyan("Type"),
                color::cyan("Branch"),
                color::cyan("Status"),
                color::cyan("Last Commit"),
                color::cyan("Health"),
            ]
            .map(|s| s.to_string())
        )
    );
    println!("{}", table_sep(&W));

    for (i, p) in latest.projects.iter().enumerate() {
        let name = p.path.split('/').next_back().unwrap_or(&p.path);
        let status = status_label(p.is_dirty, p.last_commit_date, stale_threshold);

        let health_str = format!("{}/100", p.health_score);
        let health_colored = if p.health_score >= 80 {
            color::green(&health_str)
        } else if p.health_score >= 50 {
            color::yellow(&health_str)
        } else {
            color::red(&health_str)
        };

        let commit_date = p.last_commit_date.format("%Y-%m-%d").to_string();
        let commit_colored = if is_stale(p.last_commit_date, stale_threshold) {
            color::red(&commit_date)
        } else {
            color::white(&commit_date)
        };

        let branch_display = if p.git_branch.len() > 10 {
            format!("{}…", &p.git_branch[..10])
        } else {
            p.git_branch.clone()
        };

        println!(
            "{}",
            table_row(
                &W,
                &[
                    color::blue(name),
                    p.project_type.clone(),
                    branch_display,
                    status,
                    commit_colored,
                    health_colored,
                ]
            )
        );

        if i < latest.projects.len() - 1 {
            println!("{}", table_sep(&W));
        }
    }

    println!("{}", table_bottom(&W));

    if diff {
        println!();
        println!("  {}", color::info("Changes since last snapshot:"));
        println!();
        if let Some(prev) = SnapshotStore::load_second_latest()? {
            let diffs = SnapshotStore::diff(&latest, &prev);
            if diffs.is_empty() {
                println!("  No changes detected.");
            } else {
                for d in &diffs {
                    println!(
                        "  {}: {} changed from {} to {}",
                        color::yellow(&d.path),
                        d.field,
                        d.old_value,
                        d.new_value
                    );
                }
            }
        } else {
            println!("  Only one snapshot available, nothing to diff.");
        }
    }

    Ok(())
}

fn visible_width(s: &str) -> usize {
    let mut len = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            len += 1;
        }
    }
    len
}

fn pad_right(s: &str, width: usize) -> String {
    let vw = visible_width(s);
    if vw >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - vw))
    }
}

fn table_row(widths: &[usize], cells: &[String]) -> String {
    let mut row = String::from("│");
    for (i, cell) in cells.iter().enumerate() {
        row.push(' ');
        row.push_str(&pad_right(cell, widths[i]));
        row.push_str(" │");
    }
    row
}

fn table_sep(widths: &[usize]) -> String {
    let mut s = String::from("├");
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            s.push('┼');
        }
        for _ in 0..(w + 2) {
            s.push('─');
        }
    }
    s.push('┤');
    s
}

fn table_top(widths: &[usize]) -> String {
    let mut s = String::from("┌");
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            s.push('┬');
        }
        for _ in 0..(w + 2) {
            s.push('─');
        }
    }
    s.push('┐');
    s
}

fn table_bottom(widths: &[usize]) -> String {
    let mut s = String::from("└");
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            s.push('┴');
        }
        for _ in 0..(w + 2) {
            s.push('─');
        }
    }
    s.push('┘');
    s
}

fn status_label(
    is_dirty: bool,
    last_commit: chrono::NaiveDateTime,
    stale_threshold_days: u32,
) -> String {
    if is_dirty {
        return color::yellow("dirty").to_string();
    }
    if is_stale(last_commit, stale_threshold_days) {
        return color::red("stale").to_string();
    }
    color::green("clean").to_string()
}

fn is_stale(last_commit: chrono::NaiveDateTime, stale_threshold_days: u32) -> bool {
    let now = chrono::Utc::now().naive_utc();
    let days = (now - last_commit).num_days();
    days >= stale_threshold_days as i64
}
