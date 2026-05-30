use anyhow::Result;

use crate::color;
use crate::config::Config;
use crate::snapshot::{ProjectSnapshot, SnapshotStore};

pub fn subcmd_report(
    diff: bool,
    format: Option<String>,
    sort: Option<String>,
    filters: Vec<String>,
) -> Result<()> {
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

    let mut projects = latest.projects.clone();

    if !filters.is_empty() {
        projects = apply_filters(projects, &filters, stale_threshold)?;
    }

    if let Some(s) = sort {
        projects = apply_sort(projects, &s)?;
    }

    if format == "json" {
        let mut json = serde_json::to_value(&latest)?;
        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "projects".to_string(),
                serde_json::to_value(&projects)?,
            );
        }
        println!("{}", serde_json::to_string_pretty(&json)?);
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
        for p in &projects {
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

    for (i, p) in projects.iter().enumerate() {
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

        if i < projects.len() - 1 {
            println!("{}", table_sep(&W));
        }
    }

    println!("{}", table_bottom(&W));

    if diff {
        print_diff(&latest)?;
    }

    Ok(())
}

fn print_diff(latest: &crate::snapshot::ScanSnapshot) -> Result<()> {
    println!();
    println!("  {}", color::info("Changes since last snapshot:"));
    println!();
    if let Some(prev) = SnapshotStore::load_second_latest()? {
        let diffs = SnapshotStore::diff(latest, &prev);
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
    Ok(())
}

fn apply_sort(
    mut projects: Vec<ProjectSnapshot>,
    sort_spec: &str,
) -> Result<Vec<ProjectSnapshot>> {
    let (field, descending) = if let Some(rest) = sort_spec.strip_prefix('-') {
        (rest, true)
    } else {
        (sort_spec, false)
    };

    let valid_fields = ["name", "type", "health", "loc", "branch", "last_commit"];
    if !valid_fields.contains(&field) {
        anyhow::bail!(
            "Invalid sort field: '{}'. Valid fields: {}",
            field,
            valid_fields.join(", ")
        );
    }

    let cmp = |a: &ProjectSnapshot, b: &ProjectSnapshot| -> std::cmp::Ordering {
        match field {
            "name" => a.path.cmp(&b.path),
            "type" => a.project_type.cmp(&b.project_type),
            "health" => a.health_score.cmp(&b.health_score),
            "loc" | "lines_of_code" => a.lines_of_code.cmp(&b.lines_of_code),
            "branch" => a.git_branch.cmp(&b.git_branch),
            "last_commit" => a.last_commit_date.cmp(&b.last_commit_date),
            _ => std::cmp::Ordering::Equal,
        }
    };

    if descending {
        projects.sort_by(|a, b| cmp(b, a));
    } else {
        projects.sort_by(cmp);
    }

    Ok(projects)
}

fn apply_filters(
    projects: Vec<ProjectSnapshot>,
    filters: &[String],
    _stale_threshold_days: u32,
) -> Result<Vec<ProjectSnapshot>> {
    use chrono::Utc;
    let now = Utc::now().naive_utc();

    let tags_index = crate::tags::TagsIndex::load()?;

    let mut filtered = projects;

    for filter in filters {
        let (field, op, expected) = parse_field_op_value(filter, filter)?;

        let valid_fields = [
            "name",
            "type",
            "health",
            "loc",
            "dirty",
            "branch",
            "last_commit",
            "tag",
        ];
        let valid_ops = ["eq", "gte", "lte", "gt", "lt"];

        if !valid_fields.contains(&field.as_str()) {
            anyhow::bail!(
                "Invalid filter field: '{}'. Valid fields: {}",
                field,
                valid_fields.join(", ")
            );
        }
        if !valid_ops.contains(&op.as_str()) {
            anyhow::bail!(
                "Invalid filter operator: '{}'. Valid operators: {}",
                op,
                valid_ops.join(", ")
            );
        }

        filtered.retain(|p| {
                let val = match field.as_str() {
                    "name" => Some(p.path.clone()),
                    "type" => Some(p.project_type.clone()),
                    "dirty" => Some(p.is_dirty.to_string()),
                    "branch" => Some(p.git_branch.clone()),
                    "health" => Some(p.health_score.to_string()),
                    "loc" => Some(p.lines_of_code.to_string()),
                    "last_commit" => Some(
                        (now - p.last_commit_date)
                            .num_days()
                            .to_string(),
                    ),
                    "tag" => {
                        if tags_index.has_tag(&p.path, &expected) {
                            Some("true".to_string())
                        } else {
                            Some("false".to_string())
                        }
                    }
                    _ => None,
                };
                match val {
                    Some(v) => match op.as_str() {
                        "eq" => v.eq_ignore_ascii_case(&expected),
                        "gte" => {
                            if let (Ok(a), Ok(b)) =
                                (v.parse::<f64>(), expected.parse::<f64>())
                            {
                                a >= b
                            } else {
                                v >= expected
                            }
                        }
                        "lte" => {
                            if let (Ok(a), Ok(b)) =
                                (v.parse::<f64>(), expected.parse::<f64>())
                            {
                                a <= b
                            } else {
                                v <= expected
                            }
                        }
                        "gt" => {
                            if let (Ok(a), Ok(b)) =
                                (v.parse::<f64>(), expected.parse::<f64>())
                            {
                                a > b
                            } else {
                                v > expected
                            }
                        }
                        "lt" => {
                            if let (Ok(a), Ok(b)) =
                                (v.parse::<f64>(), expected.parse::<f64>())
                            {
                                a < b
                            } else {
                                v < expected
                            }
                        }
                        _ => false,
                    },
                    None => true,
                }
            });
    }

    Ok(filtered)
}

fn parse_field_op_value(
    input: &str,
    original: &str,
) -> Result<(String, String, String)> {
    let eq_pos = input.find('=').ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid filter syntax: '{}'. Use format <field>[:<op>]=<value>",
            original
        )
    })?;

    let (key_part, value) = input.split_at(eq_pos);
    let value = value[1..].to_string();

    let (field, op) = if let Some(colon_pos) = key_part.find(':') {
        let f = key_part[..colon_pos].to_string();
        let o = key_part[colon_pos + 1..].to_string();
        (f, o)
    } else {
        (key_part.to_string(), "eq".to_string())
    };

    Ok((field, op, value))
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_project(
        path: &str,
        ptype: &str,
        health: u8,
        dirty: bool,
        loc: u32,
        branch: &str,
    ) -> ProjectSnapshot {
        let now = Utc::now().naive_utc();
        ProjectSnapshot {
            path: path.to_string(),
            project_type: ptype.to_string(),
            git_branch: branch.to_string(),
            is_dirty: dirty,
            unpushed_commits: 0,
            last_commit_date: now,
            last_modified_date: now,
            lines_of_code: loc,
            health_score: health,
        }
    }

    #[test]
    fn test_apply_sort_health_asc() {
        let projects = vec![
            make_project("/a", "Rust", 60, false, 100, "main"),
            make_project("/b", "Python", 80, false, 200, "main"),
            make_project("/c", "Rust", 70, false, 300, "main"),
        ];
        let sorted = apply_sort(projects, "health").unwrap();
        assert_eq!(sorted[0].health_score, 60);
        assert_eq!(sorted[2].health_score, 80);
    }

    #[test]
    fn test_apply_sort_health_desc() {
        let projects = vec![
            make_project("/a", "Rust", 60, false, 100, "main"),
            make_project("/b", "Python", 80, false, 200, "main"),
            make_project("/c", "Rust", 70, false, 300, "main"),
        ];
        let sorted = apply_sort(projects, "-health").unwrap();
        assert_eq!(sorted[0].health_score, 80);
        assert_eq!(sorted[2].health_score, 60);
    }

    #[test]
    fn test_apply_sort_invalid_field() {
        let projects = vec![];
        let result = apply_sort(projects, "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_filter_type_eq() {
        let projects = vec![
            make_project("/a", "Rust", 60, false, 100, "main"),
            make_project("/b", "Python", 80, false, 200, "main"),
        ];
        let filtered = apply_filters(projects, &["type=Rust".to_string()], 90).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, "/a");
    }

    #[test]
    fn test_apply_filter_health_gte() {
        let projects = vec![
            make_project("/a", "Rust", 60, false, 100, "main"),
            make_project("/b", "Python", 80, false, 200, "main"),
            make_project("/c", "Rust", 70, false, 300, "main"),
        ];
        let filtered =
            apply_filters(projects, &["health:gte=70".to_string()], 90).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_apply_filter_health_lte() {
        let projects = vec![
            make_project("/a", "Rust", 60, false, 100, "main"),
            make_project("/b", "Python", 80, false, 200, "main"),
        ];
        let filtered =
            apply_filters(projects, &["health:lte=70".to_string()], 90).unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_apply_filter_dirty_eq() {
        let projects = vec![
            make_project("/a", "Rust", 60, true, 100, "main"),
            make_project("/b", "Python", 80, false, 200, "main"),
        ];
        let filtered = apply_filters(projects, &["dirty=true".to_string()], 90).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, "/a");
    }

    #[test]
    fn test_apply_filter_type_and_health() {
        let projects = vec![
            make_project("/a", "Rust", 60, false, 100, "main"),
            make_project("/b", "Rust", 80, false, 200, "main"),
            make_project("/c", "Python", 90, false, 300, "main"),
        ];
        let filtered = apply_filters(
            projects,
            &["type=Rust".to_string(), "health:gte=70".to_string()],
            90,
        )
        .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].path, "/b");
    }

    #[test]
    fn test_parse_field_op_value_eq() {
        let (field, op, val) = parse_field_op_value("type=Rust", "type=Rust").unwrap();
        assert_eq!(field, "type");
        assert_eq!(op, "eq");
        assert_eq!(val, "Rust");
    }

    #[test]
    fn test_parse_field_op_value_gte() {
        let (field, op, val) =
            parse_field_op_value("health:gte=80", "health:gte=80").unwrap();
        assert_eq!(field, "health");
        assert_eq!(op, "gte");
        assert_eq!(val, "80");
    }

    #[test]
    fn test_parse_field_op_value_invalid() {
        let result = parse_field_op_value("noequalsign", "noequalsign");
        assert!(result.is_err());
    }
}
