use std::path::Path;

use anyhow::Result;
use chrono::TimeZone;

use crate::analyzer;
use crate::color;
use crate::config::Config;
use crate::snapshot::{format_health_deductions, ProjectSnapshot};

pub fn subcmd_inspect(path: Option<String>, format: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let path = path.unwrap_or_else(|| ".".to_string());
    let dir = Path::new(&path);

    if !dir.exists() {
        anyhow::bail!("Path not found: {}", path);
    }

    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    if !dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", path);
    }

    let snapshot = analyze_project_on_demand(dir, &config, fmt == "json")?;

    if fmt == "json" {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    }

    Ok(())
}

fn analyze_project_on_demand(
    dir: &Path,
    config: &Config,
    quiet: bool,
) -> Result<ProjectSnapshot> {
    let stale_threshold = config.report.stale_threshold_days;
    let project_type = analyzer::ProjectType::detect(dir)?;
    let is_git = dir.join(".git").exists();
    let loc = analyzer::estimate_loc(dir);

    if !is_git {
        let ftd = analyzer::file_type_distribution(dir);
        if !quiet {
            println!("{}", color::info("Not a Git repository — file info only"));
            println!();
            print_project_info(dir, &project_type, loc, &ftd);
        }
        return Ok(basic_snapshot(dir, &project_type, loc));
    }

    let git = match analyzer::git_health(dir)? {
        Some(g) => g,
        None => {
            let ftd = analyzer::file_type_distribution(dir);
            if !quiet {
                println!("  {}", color::info("Instant analysis (no snapshot found)"));
                print_project_info(dir, &project_type, loc, &ftd);
            }
            return Ok(basic_snapshot(dir, &project_type, loc));
        }
    };

    let health_score = analyzer::compute_health_score(
        git.is_dirty,
        git.unpushed_commits,
        git.last_commit_date,
        std::fs::metadata(dir)
            .and_then(|m| m.modified())
            .map(|sys_time| {
                let duration = sys_time
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = duration.as_secs() as i64;
                chrono::Utc
                    .timestamp_opt(secs, 0)
                    .single()
                    .map(|dt| dt.naive_utc())
                    .unwrap_or_default()
            })
            .unwrap_or_default(),
        loc,
        stale_threshold,
    );

    if !quiet {
        print_full_project_info(dir, &project_type, &git, health_score, loc, stale_threshold)?;
    }

    Ok(ProjectSnapshot {
        path: dir.to_string_lossy().to_string(),
        project_type: project_type.as_str().to_string(),
        git_branch: git.branch,
        is_dirty: git.is_dirty,
        unpushed_commits: git.unpushed_commits,
        last_commit_date: git.last_commit_date,
        last_modified_date: chrono::NaiveDateTime::default(),
        lines_of_code: loc,
        health_score,
    })
}

fn basic_snapshot(dir: &Path, project_type: &analyzer::ProjectType, loc: u32) -> ProjectSnapshot {
    ProjectSnapshot {
        path: dir.to_string_lossy().to_string(),
        project_type: project_type.as_str().to_string(),
        git_branch: String::new(),
        is_dirty: false,
        unpushed_commits: 0,
        last_commit_date: chrono::NaiveDateTime::default(),
        last_modified_date: chrono::NaiveDateTime::default(),
        lines_of_code: loc,
        health_score: 0,
    }
}

fn print_project_info(
    dir: &Path,
    project_type: &analyzer::ProjectType,
    loc: u32,
    ftd: &analyzer::FileTypeDistribution,
) {
    println!("  Path:    {}", color::cyan(&dir.to_string_lossy()));
    println!("  Type:    {}", project_type.as_str());
    println!("  LOC:     {}", loc);
    println!();
    if !ftd.groups.is_empty() {
        println!("  File type distribution:");
        for (name, count, pct) in &ftd.groups {
            println!("    {:30} {:>5} files ({:>2}%)", name, count, pct);
        }
    }
}

fn print_full_project_info(
    dir: &Path,
    project_type: &analyzer::ProjectType,
    git: &analyzer::GitHealth,
    health_score: u8,
    loc: u32,
    stale_threshold: u32,
) -> Result<()> {
    let commit_stats = analyzer::count_commits(dir)?;
    let ftd = analyzer::file_type_distribution(dir);
    let extra = analyzer::git_extra_health(dir)?;

    println!("  Path:     {}", color::cyan(&dir.to_string_lossy()));
    println!("  Type:     {}", project_type.as_str());
    println!("  Branch:   {}", git.branch);
    println!("  Health:   {}/100", health_score);
    println!("  LOC:      {}", loc);
    println!("  Dirty:    {}", if git.is_dirty { "yes" } else { "no" });
    println!("  Unpushed: {}", git.unpushed_commits);
    println!();

    if let Some(ref cs) = commit_stats {
        println!("  Commit activity:");
        println!("    Total:       {}", cs.total);
        println!("    Last 30d:    {}", cs.last_30_days);
        println!("    Last 90d:    {}", cs.last_90_days);
        println!("    Last year:   {}", cs.last_year);
        println!("    Authors:     {}", cs.authors);
        println!();
    }

    if !ftd.groups.is_empty() {
        println!("  File type distribution:");
        for (name, count, pct) in &ftd.groups {
            println!("    {:30} {:>5} files ({:>2}%)", name, count, pct);
        }
        println!();
    }

    if let Some(ref ex) = extra {
        println!("  Extra health:");
        println!("    Stash count:     {}", ex.stash_count);
        println!("    Untracked files: {}", ex.untracked_files);
        println!("    Behind upstream: {}", ex.behind_upstream);
        println!();
    }

    let deductions = format_health_deductions(git.is_dirty, git.last_commit_date, loc, stale_threshold);
    if !deductions.is_empty() {
        println!("  Health deductions: {}", deductions.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_invalid_format() {
        let result = subcmd_inspect(Some(".".to_string()), Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported format"));
    }

    #[test]
    fn test_inspect_missing_path() {
        let result = subcmd_inspect(Some("/nonexistent_path_projector_42".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_inspect_not_a_directory() {
        let result = subcmd_inspect(Some("Cargo.toml".to_string()), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_basic_snapshot_construction() {
        let dir = Path::new("/tmp");
        let pt = analyzer::ProjectType::Unknown;
        let snap = basic_snapshot(dir, &pt, 42);
        assert_eq!(snap.path, "/tmp");
        assert_eq!(snap.project_type, "Unknown");
        assert_eq!(snap.lines_of_code, 42);
        assert_eq!(snap.health_score, 0);
        assert!(!snap.is_dirty);
    }
}
