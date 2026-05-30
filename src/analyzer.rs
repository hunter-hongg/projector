use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::{NaiveDateTime, TimeZone, Utc};
use git2::Repository;

use crate::snapshot::ProjectSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    JavaScript,
    Go,
    Python,
    JavaKotlin,
    Cpp,
    OCaml,
    Dart,
    Unknown,
}

impl ProjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Rust => "Rust",
            ProjectType::JavaScript => "JavaScript/TypeScript",
            ProjectType::Go => "Go",
            ProjectType::Python => "Python",
            ProjectType::JavaKotlin => "Java/Kotlin",
            ProjectType::Cpp => "C/C++",
            ProjectType::OCaml => "OCaml",
            ProjectType::Dart => "Dart",
            ProjectType::Unknown => "Unknown",
        }
    }

    pub fn detect(dir: &Path) -> Result<Self> {
        let filenames = read_filenames(dir)?;

        if filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("Cargo.toml"))
        {
            Ok(ProjectType::Rust)
        } else if filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("package.json"))
        {
            Ok(ProjectType::JavaScript)
        } else if filenames.iter().any(|f| f.eq_ignore_ascii_case("go.mod")) {
            Ok(ProjectType::Go)
        } else if filenames.iter().any(|f| {
            f.eq_ignore_ascii_case("requirements.txt")
                || f.eq_ignore_ascii_case("setup.py")
                || f.eq_ignore_ascii_case("pyproject.toml")
        }) {
            Ok(ProjectType::Python)
        } else if filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("build.gradle") || f.eq_ignore_ascii_case("pom.xml"))
        {
            Ok(ProjectType::JavaKotlin)
        } else if filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("CMakeLists.txt"))
        {
            Ok(ProjectType::Cpp)
        } else if filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("dune-project"))
        {
            Ok(ProjectType::OCaml)
        } else if filenames
            .iter()
            .any(|f| f.eq_ignore_ascii_case("pubspec.yaml"))
        {
            Ok(ProjectType::Dart)
        } else {
            Ok(detect_by_extensions(dir))
        }
    }
}

fn detect_by_extensions(dir: &Path) -> ProjectType {
    let mut counts = HashMap::new();
    scan_dir_for_extensions(dir, &mut counts);

    let rust = counts.get("rs").copied().unwrap_or(0);
    let js = counts.get("js").copied().unwrap_or(0)
        + counts.get("ts").copied().unwrap_or(0)
        + counts.get("jsx").copied().unwrap_or(0)
        + counts.get("tsx").copied().unwrap_or(0);
    let go = counts.get("go").copied().unwrap_or(0);
    let py = counts.get("py").copied().unwrap_or(0);
    let java_kt = counts.get("java").copied().unwrap_or(0)
        + counts.get("kt").copied().unwrap_or(0)
        + counts.get("kts").copied().unwrap_or(0);
    let cpp = counts.get("c").copied().unwrap_or(0)
        + counts.get("h").copied().unwrap_or(0)
        + counts.get("cpp").copied().unwrap_or(0)
        + counts.get("hpp").copied().unwrap_or(0)
        + counts.get("cc").copied().unwrap_or(0)
        + counts.get("cxx").copied().unwrap_or(0);
    let ocaml = counts.get("ml").copied().unwrap_or(0) + counts.get("mli").copied().unwrap_or(0);
    let dart = counts.get("dart").copied().unwrap_or(0);

    let candidates = [
        (rust, ProjectType::Rust),
        (js, ProjectType::JavaScript),
        (go, ProjectType::Go),
        (py, ProjectType::Python),
        (java_kt, ProjectType::JavaKotlin),
        (cpp, ProjectType::Cpp),
        (ocaml, ProjectType::OCaml),
        (dart, ProjectType::Dart),
    ];

    let max_count = candidates
        .iter()
        .map(|(c, _)| c)
        .max()
        .copied()
        .unwrap_or(0);
    if max_count == 0 {
        return ProjectType::Unknown;
    }
    let top: Vec<_> = candidates.iter().filter(|(c, _)| *c == max_count).collect();
    if top.len() == 1 {
        return top[0].1.clone();
    }
    ProjectType::Unknown
}

fn scan_dir_for_extensions(dir: &Path, counts: &mut HashMap<String, u32>) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                    stack.push(path);
                } else if path.is_file()
                    && let Some(ext) = path.extension().and_then(|e| e.to_str())
                {
                    *counts.entry(ext.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
}

fn read_filenames(dir: &Path) -> Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(dir)?;
    let names = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    Ok(names)
}

/// 轻量检查：目录是否包含 `.git` 子目录
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// 遍历 `dir` 的子目录，按是否为 git 项目分类。
/// `skip_hidden=true` 时跳过以 `.` 开头的目录。
pub fn classify_dirs(
    dir: &Path,
    skip_hidden: bool,
) -> Result<(Vec<std::path::PathBuf>, Vec<std::path::PathBuf>)> {
    let mut projects = Vec::new();
    let mut others = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if skip_hidden {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
        }
        if is_git_repo(&path) {
            projects.push(path);
        } else {
            others.push(path);
        }
    }

    Ok((projects, others))
}

pub struct GitHealth {
    pub branch: String,
    pub is_dirty: bool,
    pub unpushed_commits: u32,
    pub last_commit_date: NaiveDateTime,
}

pub fn git_health(dir: &Path) -> Result<Option<GitHealth>> {
    let repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return Ok(Some(GitHealth {
                branch: "HEAD (no commits)".to_string(),
                is_dirty: false,
                unpushed_commits: 0,
                last_commit_date: NaiveDateTime::default(),
            }));
        }
    };

    let branch = head.shorthand().unwrap_or("unknown").to_string();

    let is_dirty = {
        let statuses = repo.statuses(None)?;
        statuses.iter().any(|s| s.status() != git2::Status::CURRENT)
    };

    let unpushed_commits = count_unpublished(&repo, &head)?;

    let last_commit_date = head
        .peel_to_commit()
        .ok()
        .map(|c| {
            let time = c.time();
            let secs = time.seconds();
            Utc.timestamp_opt(secs, 0)
                .single()
                .map(|dt| dt.naive_utc())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    Ok(Some(GitHealth {
        branch,
        is_dirty,
        unpushed_commits,
        last_commit_date,
    }))
}

fn count_unpublished(repo: &Repository, head: &git2::Reference) -> Result<u32> {
    let branch_name = match head.shorthand() {
        Some(name) => name,
        None => return Ok(0),
    };

    let upstream_name = format!("refs/remotes/origin/{}", branch_name);
    let upstream = match repo.find_reference(&upstream_name) {
        Ok(r) => r,
        Err(_) => return Ok(0),
    };

    let local_commit = match head.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    let upstream_commit = match upstream.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };

    let merge_base = match repo.merge_base(local_commit.id(), upstream_commit.id()) {
        Ok(id) => id,
        Err(_) => return Ok(0),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.push(local_commit.id())?;
    revwalk.hide(merge_base)?;

    Ok(revwalk.count() as u32)
}

const COUNTABLE_EXTENSIONS: &[&str] = &[
    "rs", "js", "ts", "jsx", "tsx", "go", "py", "java", "kt", "kts", "c", "h", "cpp", "hpp", "cc",
    "cxx", "ml", "mli", "dart", "toml", "json", "yaml", "yml", "md", "css", "html",
];

pub fn estimate_loc(dir: &Path) -> u32 {
    let mut total = 0u32;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                    stack.push(path);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && COUNTABLE_EXTENSIONS.contains(&ext)
                    && let Ok(content) = fs::read_to_string(&path)
                {
                    total += content.lines().count() as u32;
                }
            }
        }
    }
    total
}

pub fn compute_health_score(
    is_dirty: bool,
    unpushed_commits: u32,
    last_commit_date: NaiveDateTime,
    last_modified_date: NaiveDateTime,
    lines_of_code: u32,
    stale_threshold_days: u32,
) -> u8 {
    let now = Utc::now().naive_utc();
    let mut score: i32 = 100;

    let days_since_commit = (now - last_commit_date).num_days();
    if days_since_commit >= stale_threshold_days as i64 {
        score -= 15;
    }

    if is_dirty {
        score -= 10;
    }

    if unpushed_commits > 0 {
        score -= ((unpushed_commits as i32) / 5) * 5;
    }

    let days_since_modified = (now - last_modified_date).num_days();
    if days_since_modified >= 60 {
        score -= 10;
    }

    if lines_of_code < 100 {
        score -= 5;
    }

    score.clamp(0, 100) as u8
}

pub fn analyze_project(dir: &Path, stale_threshold_days: u32) -> Result<Option<ProjectSnapshot>> {
    let project_type = ProjectType::detect(dir)?;

    let git = match git_health(dir)? {
        Some(g) => g,
        None => return Ok(None),
    };

    let lines_of_code = estimate_loc(dir);

    let last_modified_date = fs::metadata(dir)
        .and_then(|m| m.modified())
        .map(|sys_time| {
            let duration = sys_time
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = duration.as_secs() as i64;
            Utc.timestamp_opt(secs, 0)
                .single()
                .map(|dt| dt.naive_utc())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    let health_score = compute_health_score(
        git.is_dirty,
        git.unpushed_commits,
        git.last_commit_date,
        last_modified_date,
        lines_of_code,
        stale_threshold_days,
    );

    Ok(Some(ProjectSnapshot {
        path: dir.to_string_lossy().to_string(),
        project_type: project_type.as_str().to_string(),
        git_branch: git.branch,
        is_dirty: git.is_dirty,
        unpushed_commits: git.unpushed_commits,
        last_commit_date: git.last_commit_date,
        last_modified_date,
        lines_of_code,
        health_score,
    }))
}

pub struct CommitStats {
    pub total: u32,
    pub last_30_days: u32,
    pub last_90_days: u32,
    pub last_year: u32,
    pub authors: u32,
}

pub fn count_commits(dir: &Path) -> Result<Option<CommitStats>> {
    let repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let mut total = 0u32;
    let mut last_30 = 0u32;
    let mut last_90 = 0u32;
    let mut last_year = 0u32;
    let mut authors = std::collections::HashSet::new();

    let now = Utc::now().naive_utc();

    let mut revwalk = match repo.revwalk() {
        Ok(w) => w,
        Err(_) => {
            return Ok(Some(CommitStats {
                total: 0,
                last_30_days: 0,
                last_90_days: 0,
                last_year: 0,
                authors: 0,
            }));
        }
    };

    if revwalk.push_head().is_err() {
        return Ok(Some(CommitStats {
            total: 0,
            last_30_days: 0,
            last_90_days: 0,
            last_year: 0,
            authors: 0,
        }));
    }

    let _ = revwalk.set_sorting(git2::Sort::TIME);

    for oid in revwalk.flatten() {
        if let Ok(commit) = repo.find_commit(oid) {
            total += 1;
            if let Some(sig) = commit.author().name() {
                authors.insert(sig.to_string());
            }
            let secs = commit.time().seconds();
            if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
                let days = (now - dt.naive_utc()).num_days();
                if days <= 30 {
                    last_30 += 1;
                }
                if days <= 90 {
                    last_90 += 1;
                }
                if days <= 365 {
                    last_year += 1;
                }
            }
        }
    }

    Ok(Some(CommitStats {
        total,
        last_30_days: last_30,
        last_90_days: last_90,
        last_year,
        authors: authors.len() as u32,
    }))
}

pub struct FileTypeDistribution {
    pub groups: Vec<(String, u32, u32)>,
}

pub fn file_type_distribution(dir: &Path) -> FileTypeDistribution {
    let mut ext_counts: HashMap<String, u32> = HashMap::new();
    scan_dir_for_extensions(dir, &mut ext_counts);

    let total_files: u32 = ext_counts.values().sum();

    let type_groups: Vec<(&str, Vec<&str>)> = vec![
        ("Rust", vec!["rs"]),
        ("JavaScript/TypeScript", vec!["js", "ts", "jsx", "tsx"]),
        ("Go", vec!["go"]),
        ("Python", vec!["py"]),
        ("Java/Kotlin", vec!["java", "kt", "kts"]),
        ("C/C++", vec!["c", "h", "cpp", "hpp", "cc", "cxx"]),
        ("OCaml", vec!["ml", "mli"]),
        ("Dart", vec!["dart"]),
        ("Data/Config", vec!["json", "yaml", "yml", "toml"]),
        ("Markdown", vec!["md"]),
        ("Web", vec!["css", "html"]),
        ("Other", vec![]),
    ];

    let mut groups: Vec<(String, u32, u32)> = Vec::new();
    let mut accounted: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (group_name, exts) in &type_groups {
        if exts.is_empty() {
            continue;
        }
        let mut count = 0u32;
        for ext in exts {
            if let Some(&c) = ext_counts.get(*ext) {
                count += c;
                accounted.insert(ext.to_string());
            }
        }
        if count > 0 {
            let pct = if total_files > 0 {
                (count as f64 / total_files as f64 * 100.0).round() as u32
            } else {
                0
            };
            groups.push((group_name.to_string(), count, pct));
        }
    }

    let other_count: u32 = ext_counts
        .iter()
        .filter(|(k, _)| !accounted.contains(*k))
        .map(|(_, v)| v)
        .sum();
    if other_count > 0 {
        let pct = if total_files > 0 {
            (other_count as f64 / total_files as f64 * 100.0).round() as u32
        } else {
            0
        };
        groups.push(("Other".to_string(), other_count, pct));
    }

    groups.sort_by_key(|g| std::cmp::Reverse(g.1));

    FileTypeDistribution { groups }
}

pub struct GitExtraHealth {
    pub stash_count: u32,
    pub untracked_files: u32,
    pub behind_upstream: u32,
}

pub fn git_extra_health(dir: &Path) -> Result<Option<GitExtraHealth>> {
    let mut repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let stash_count = {
        let mut count = 0u32;
        let _ = repo.stash_foreach(|_, _, _| {
            count += 1;
            true
        });
        count
    };

    let (untracked_files, behind_upstream) = {
        let statuses = repo.statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(true),
        ))?;

        let untracked = statuses
            .iter()
            .filter(|s| s.status() == git2::Status::WT_NEW)
            .count() as u32;

        let behind = if let Ok(head) = repo.head() {
            if let Some(name) = head.shorthand() {
                let upstream_name = format!("refs/remotes/origin/{}", name);
                if let Ok(upstream) = repo.find_reference(&upstream_name) {
                    if let (Ok(local), Ok(remote)) =
                        (head.peel_to_commit(), upstream.peel_to_commit())
                    {
                        if let Ok(merge_base) = repo.merge_base(local.id(), remote.id()) {
                            let mut revwalk = repo.revwalk().ok();
                            if let Some(ref mut w) = revwalk {
                                let _ = w.push(remote.id());
                                let _ = w.hide(merge_base);
                                w.count() as u32
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        (untracked, behind)
    };

    Ok(Some(GitExtraHealth {
        stash_count,
        untracked_files,
        behind_upstream,
    }))
}

pub struct ProjectStats {
    pub total_projects: usize,
    pub type_distribution: Vec<(String, usize, f64)>,
    pub avg_health: f64,
    pub median_health: f64,
    pub std_dev_health: f64,
    pub health_buckets: (usize, usize, usize),
    pub top5: Vec<ProjectSnapshot>,
    pub bottom5: Vec<ProjectSnapshot>,
    pub total_loc: u32,
    pub dirty_ratio: f64,
    pub stale_ratio: f64,
}

pub fn compute_stats(
    snapshot: &crate::snapshot::ScanSnapshot,
    stale_threshold_days: u32,
) -> ProjectStats {
    use chrono::Utc;

    let now = Utc::now().naive_utc();
    let total = snapshot.projects.len();

    let mut type_map: HashMap<String, usize> = HashMap::new();
    for p in &snapshot.projects {
        *type_map.entry(p.project_type.clone()).or_insert(0) += 1;
    }
    let type_distribution: Vec<(String, usize, f64)> = type_map
        .into_iter()
        .map(|(k, v)| {
            let pct = if total > 0 {
                (v as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            (k, v, pct)
        })
        .collect();

    let mut sorted: Vec<&ProjectSnapshot> = snapshot.projects.iter().collect();
    sorted.sort_by_key(|p| p.health_score);

    let avg_health = if total > 0 {
        sorted.iter().map(|p| p.health_score as f64).sum::<f64>() / total as f64
    } else {
        0.0
    };

    let median_health = if total > 0 {
        let mid = total / 2;
        if total.is_multiple_of(2) {
            (sorted[mid - 1].health_score as f64 + sorted[mid].health_score as f64) / 2.0
        } else {
            sorted[mid].health_score as f64
        }
    } else {
        0.0
    };

    let std_dev_health = if total > 1 {
        let variance = sorted
            .iter()
            .map(|p| {
                let diff = p.health_score as f64 - avg_health;
                diff * diff
            })
            .sum::<f64>()
            / (total - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let high = sorted.iter().filter(|p| p.health_score >= 80).count();
    let mid = sorted
        .iter()
        .filter(|p| p.health_score >= 50 && p.health_score < 80)
        .count();
    let low = sorted.iter().filter(|p| p.health_score < 50).count();

    let top5: Vec<ProjectSnapshot> = sorted.iter().rev().take(5).map(|p| (*p).clone()).collect();
    let bottom5: Vec<ProjectSnapshot> = sorted.iter().take(5).map(|p| (*p).clone()).collect();

    let total_loc: u32 = snapshot.projects.iter().map(|p| p.lines_of_code).sum();
    let dirty_count = snapshot.projects.iter().filter(|p| p.is_dirty).count();
    let dirty_ratio = if total > 0 {
        dirty_count as f64 / total as f64
    } else {
        0.0
    };

    let stale_count = snapshot
        .projects
        .iter()
        .filter(|p| {
            let days = (now - p.last_commit_date).num_days();
            days >= stale_threshold_days as i64
        })
        .count();
    let stale_ratio = if total > 0 {
        stale_count as f64 / total as f64
    } else {
        0.0
    };

    ProjectStats {
        total_projects: total,
        type_distribution,
        avg_health,
        median_health,
        std_dev_health,
        health_buckets: (high, mid, low),
        top5,
        bottom5,
        total_loc,
        dirty_ratio,
        stale_ratio,
    }
}

pub struct TrendPoint {
    pub date: String,
    pub value: f64,
}

pub fn draw_ascii_chart(
    points: &[TrendPoint],
    width: usize,
    height: usize,
) -> Vec<String> {
    if points.is_empty() {
        return vec!["(no data)".to_string()];
    }

    let min_val = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let max_val = points.iter().map(|p| p.value).fold(f64::NEG_INFINITY, f64::max);

    if (max_val - min_val).abs() < f64::EPSILON {
        return vec![format!("all values = {:.1}", min_val)];
    }

    let plot_width = width.saturating_sub(8).max(2);
    let plot_height = height.saturating_sub(2).max(2);

    let mut lines = Vec::new();

    for row in 0..plot_height {
        let ratio = 1.0 - (row as f64 / (plot_height - 1) as f64);
        let val = min_val + ratio * (max_val - min_val);

        let label = if val == min_val || (val - max_val).abs() < (max_val - min_val) * 0.05 {
            format!("{:.0}", val)
        } else {
            String::new()
        };

        let padded_label = if row % 2 == 0 || !label.is_empty() {
            format!("{:>6} ", label)
        } else {
            "       ".to_string()
        };

        let mut row_chars = String::with_capacity(plot_width);
        for col in 0..plot_width {
            let point_idx = (col as f64 / (plot_width - 1) as f64 * (points.len() - 1) as f64)
                .round() as usize;
            let point_val = points[point_idx].value;
            let point_ratio = (point_val - min_val) / (max_val - min_val);

            let y_pos = (plot_height - 1) as f64 * (1.0 - point_ratio);
            let distance = (row as f64 - y_pos).abs();

            if distance < 0.5 {
                row_chars.push('●');
            } else if col > 0 {
                let prev_idx = ((col - 1) as f64 / (plot_width - 1) as f64
                    * (points.len() - 1) as f64)
                    .round() as usize;
                let prev_y = (plot_height - 1) as f64
                    * (1.0 - (points[prev_idx].value - min_val) / (max_val - min_val));
                let row_f = row as f64;
                if (row_f > prev_y && row_f < y_pos)
                    || (row_f < prev_y && row_f > y_pos)
                {
                    row_chars.push('│');
                } else {
                    row_chars.push(' ');
                }
            } else {
                row_chars.push(' ');
            }
        }

        lines.push(format!("{}{}", padded_label, row_chars));
    }

    let x_axis = format!("       {}", "─".repeat(plot_width));
    lines.push(x_axis);

    let x_labels = self::format_x_axis_labels(points, plot_width);
    lines.push(format!("       {}", x_labels));

    lines
}

fn format_x_axis_labels(points: &[TrendPoint], width: usize) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut labels = vec![" ".to_string(); width];
    if let Some(first) = points.first() {
        let d = &first.date;
        for (i, c) in d.chars().enumerate() {
            if i < width {
                labels[i] = c.to_string();
            }
        }
    }
    if let Some(last) = points.last() {
        let d = &last.date;
        let start = width.saturating_sub(d.len());
        for (i, c) in d.chars().enumerate() {
            let pos = start + i;
            if pos < width {
                labels[pos] = c.to_string();
            }
        }
    }
    labels.concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_health_score_perfect() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(false, 0, now, now, 500, 90);
        assert_eq!(score, 100);
    }

    #[test]
    fn test_health_score_dirty() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(true, 0, now, now, 500, 90);
        assert_eq!(score, 90);
    }

    #[test]
    fn test_health_score_unpushed_commits() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(false, 5, now, now, 500, 90);
        assert_eq!(score, 95);
    }

    #[test]
    fn test_health_score_stale() {
        let now = Utc::now().naive_utc();
        let old = Utc.timestamp_opt(0, 0).single().unwrap().naive_utc();
        let score = compute_health_score(false, 0, old, now, 500, 90);
        assert!(score < 100);
        assert_eq!(score, 85);
    }

    #[test]
    fn test_health_score_floor() {
        let old = Utc.timestamp_opt(0, 0).single().unwrap().naive_utc();
        let score = compute_health_score(true, 200, old, old, 50, 30);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_health_score_small_project() {
        let now = Utc::now().naive_utc();
        let score = compute_health_score(false, 0, now, now, 50, 90);
        assert_eq!(score, 95);
    }

    #[test]
    fn test_project_type_detect_rust() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pt = ProjectType::detect(dir).unwrap();
        assert_eq!(pt, ProjectType::Rust);
    }

    #[test]
    fn test_project_type_as_str() {
        assert_eq!(ProjectType::Rust.as_str(), "Rust");
        assert_eq!(ProjectType::JavaScript.as_str(), "JavaScript/TypeScript");
        assert_eq!(ProjectType::Unknown.as_str(), "Unknown");
    }

    #[test]
    fn test_project_type_detect_unknown() {
        let dir = Path::new("/nonexistent_path_42");
        let pt = ProjectType::detect(dir).unwrap();
        assert_eq!(pt, ProjectType::Unknown);
    }

    #[test]
    fn test_read_filenames() {
        let dir = std::env::temp_dir().join("projector_test_read_filenames");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("Cargo.toml"), "").unwrap();
        std::fs::write(dir.join("main.rs"), "").unwrap();
        let names = read_filenames(&dir).unwrap();
        assert!(names.contains(&"Cargo.toml".to_string()));
        assert!(names.contains(&"main.rs".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_detect_by_extensions_tie_returns_unknown() {
        let dir = std::env::temp_dir().join("projector_test_tie");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "").unwrap();
        std::fs::write(dir.join("main.go"), "").unwrap();
        let pt = ProjectType::detect(&dir).unwrap();
        assert_eq!(pt, ProjectType::Unknown);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_estimate_loc_iterative() {
        let dir = std::env::temp_dir().join("projector_test_loc");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.rs"), "line1\nline2\n").unwrap();
        std::fs::write(dir.join("b.py"), "x\n").unwrap();
        assert_eq!(estimate_loc(&dir), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_file_type_distribution_empty_dir() {
        let dir = std::env::temp_dir().join("projector_test_ftd_empty");
        let _ = std::fs::create_dir_all(&dir);
        let ftd = file_type_distribution(&dir);
        assert!(ftd.groups.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_file_type_distribution_with_files() {
        let dir = std::env::temp_dir().join("projector_test_ftd");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "").unwrap();
        std::fs::write(dir.join("lib.rs"), "").unwrap();
        std::fs::write(dir.join("style.css"), "").unwrap();
        let ftd = file_type_distribution(&dir);
        assert!(!ftd.groups.is_empty());
        let rust_count = ftd
            .groups
            .iter()
            .find(|(name, _, _)| name == "Rust")
            .map(|(_, count, _)| *count)
            .unwrap_or(0);
        assert_eq!(rust_count, 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compute_stats_empty() {
        let snapshot = crate::snapshot::ScanSnapshot {
            timestamp: Utc::now().naive_utc(),
            scanned_path: ".".to_string(),
            projects: vec![],
        };
        let stats = compute_stats(&snapshot, 90);
        assert_eq!(stats.total_projects, 0);
        assert_eq!(stats.avg_health, 0.0);
        assert_eq!(stats.median_health, 0.0);
        assert_eq!(stats.std_dev_health, 0.0);
        assert_eq!(stats.total_loc, 0);
    }

    #[test]
    fn test_compute_stats_single_project() {
        let now = Utc::now().naive_utc();
        let p = ProjectSnapshot {
            path: "/test".to_string(),
            project_type: "Rust".to_string(),
            git_branch: "main".to_string(),
            is_dirty: false,
            unpushed_commits: 0,
            last_commit_date: now,
            last_modified_date: now,
            lines_of_code: 500,
            health_score: 85,
        };
        let snapshot = crate::snapshot::ScanSnapshot {
            timestamp: now,
            scanned_path: ".".to_string(),
            projects: vec![p],
        };
        let stats = compute_stats(&snapshot, 90);
        assert_eq!(stats.total_projects, 1);
        assert!((stats.avg_health - 85.0).abs() < 0.001);
        assert!((stats.median_health - 85.0).abs() < 0.001);
        assert!((stats.std_dev_health - 0.0).abs() < 0.001);
        assert_eq!(stats.total_loc, 500);
    }

    #[test]
    fn test_compute_stats_multiple_projects() {
        let now = Utc::now().naive_utc();
        let projects = vec![
            ProjectSnapshot {
                path: "/a".to_string(),
                project_type: "Rust".to_string(),
                git_branch: "main".to_string(),
                is_dirty: false,
                unpushed_commits: 0,
                last_commit_date: now,
                last_modified_date: now,
                lines_of_code: 100,
                health_score: 100,
            },
            ProjectSnapshot {
                path: "/b".to_string(),
                project_type: "Python".to_string(),
                git_branch: "main".to_string(),
                is_dirty: true,
                unpushed_commits: 0,
                last_commit_date: now,
                last_modified_date: now,
                lines_of_code: 200,
                health_score: 80,
            },
            ProjectSnapshot {
                path: "/c".to_string(),
                project_type: "Rust".to_string(),
                git_branch: "main".to_string(),
                is_dirty: false,
                unpushed_commits: 0,
                last_commit_date: now,
                last_modified_date: now,
                lines_of_code: 300,
                health_score: 60,
            },
        ];
        let snapshot = crate::snapshot::ScanSnapshot {
            timestamp: now,
            scanned_path: ".".to_string(),
            projects,
        };
        let stats = compute_stats(&snapshot, 90);
        assert_eq!(stats.total_projects, 3);
        assert!((stats.avg_health - 80.0).abs() < 0.001);
        assert!((stats.median_health - 80.0).abs() < 0.001);
        assert_eq!(stats.total_loc, 600);
        assert!((stats.dirty_ratio - 1.0 / 3.0).abs() < 0.001);
    }
}
