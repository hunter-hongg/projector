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
}
