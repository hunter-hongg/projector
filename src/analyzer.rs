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
        if has_file(dir, "Cargo.toml")? {
            Ok(ProjectType::Rust)
        } else if has_file(dir, "package.json")? {
            Ok(ProjectType::JavaScript)
        } else if has_file(dir, "go.mod")? {
            Ok(ProjectType::Go)
        } else if has_file(dir, "requirements.txt")? || has_file(dir, "setup.py")? || has_file(dir, "pyproject.toml")? {
            Ok(ProjectType::Python)
        } else if has_file(dir, "build.gradle")? || has_file(dir, "pom.xml")? {
            Ok(ProjectType::JavaKotlin)
        } else if has_file(dir, "CMakeLists.txt")? {
            Ok(ProjectType::Cpp)
        } else if has_file(dir, "dune-project")? {
            Ok(ProjectType::OCaml)
        } else if has_file(dir, "pubspec.yaml")? {
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
    let ocaml = counts.get("ml").copied().unwrap_or(0)
        + counts.get("mli").copied().unwrap_or(0);
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

    candidates
        .into_iter()
        .max_by_key(|(count, _)| *count)
        .filter(|(count, _)| *count > 0)
        .map(|(_, pt)| pt)
        .unwrap_or(ProjectType::Unknown)
}

fn scan_dir_for_extensions(dir: &Path, counts: &mut HashMap<String, u32>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
                scan_dir_for_extensions(&path, counts);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    *counts.entry(ext.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
}

fn has_file(dir: &Path, file: &str) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if name.eq_ignore_ascii_case(file) {
                return Ok(true);
            }
        }
    }
    Ok(false)
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

    let last_commit_date = head.peel_to_commit()
        .ok()
        .map(|c| {
            let time = c.time();
            let secs = time.seconds();
            Utc.timestamp_opt(secs, 0)
                .single()
                .map(|dt| dt.naive_utc())
                .unwrap_or(NaiveDateTime::default())
        })
        .unwrap_or(NaiveDateTime::default());

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

    let mut revwalk = match repo.revwalk() {
        Ok(w) => w,
        Err(_) => return Ok(0),
    };

    let _ = revwalk.push(local_commit.id());
    let _ = revwalk.hide(merge_base);

    Ok(revwalk.count() as u32)
}

pub fn estimate_loc(dir: &Path) -> u32 {
    let extensions = [
        "rs", "js", "ts", "jsx", "tsx", "go", "py", "java", "kt", "kts",
        "c", "h", "cpp", "hpp", "cc", "cxx", "ml", "mli", "dart",
        "toml", "json", "yaml", "yml", "md", "css", "html",
    ];
    let mut total = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
                total += estimate_loc(&path);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            total += content.lines().count() as u32;
                        }
                    }
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

    let git = git_health(dir)?;

    let has_git = dir.join(".git").is_dir();

    if !has_git {
        return Ok(None);
    }

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
                .unwrap_or(NaiveDateTime::default())
        })
        .unwrap_or(NaiveDateTime::default());

    let (git_branch, is_dirty, unpushed_commits, last_commit_date) = match git {
        Some(h) => (h.branch, h.is_dirty, h.unpushed_commits, h.last_commit_date),
        None => ("no git history".to_string(), false, 0, last_modified_date),
    };

    let health_score = compute_health_score(
        is_dirty,
        unpushed_commits,
        last_commit_date,
        last_modified_date,
        lines_of_code,
        stale_threshold_days,
    );

    Ok(Some(ProjectSnapshot {
        path: dir.to_string_lossy().to_string(),
        project_type: project_type.as_str().to_string(),
        git_branch,
        is_dirty,
        unpushed_commits,
        last_commit_date,
        last_modified_date,
        lines_of_code,
        health_score,
    }))
}
