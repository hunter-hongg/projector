use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

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

pub(crate) fn walk_dirs<F>(dir: &Path, mut visit_file: F)
where
    F: FnMut(&std::path::Path),
{
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
                } else if path.is_file() {
                    visit_file(&path);
                }
            }
        }
    }
}

pub(crate) fn scan_dir_for_extensions(dir: &Path, counts: &mut HashMap<String, u32>) {
    walk_dirs(dir, |path| {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            *counts.entry(ext.to_string()).or_insert(0) += 1;
        }
    });
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

/// Lightweight check: whether the directory contains a `.git` subdirectory
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Walk `dir`'s subdirectories and classify each as a git project or not.
/// When `skip_hidden=true`, directories starting with `.` are skipped.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
