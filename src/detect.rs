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

/// Directories never descended into while classifying, besides dotfiles.
pub const SKIP_DIR_NAMES: &[&str] = &["node_modules", "target"];

/// Discovered projects as `(path, depth)` plus the plain directories inspected
/// but not classified as projects.
pub type ClassifiedDirs = (Vec<(std::path::PathBuf, u32)>, Vec<std::path::PathBuf>);

/// Hard ceiling on recursion, guarding against pathological or looping trees
/// when `max_depth` is set to 0 (unlimited).
const DEPTH_CEILING: usize = 32;

/// Walk `dir` and classify its subdirectories as git projects or plain
/// directories, descending up to `max_depth` levels (`0` = unlimited).
///
/// Returns projects as `(path, depth)` where depth `1` is a direct child of
/// `dir`, plus the plain directories that were inspected but are not projects.
/// A discovered repo is not descended into, so nested repos inside a repo are
/// not reported. Symlinked directories are classified but never descended into,
/// so a link cycle cannot drive the walk. When `skip_hidden` is true, directories
/// starting with `.` are ignored at every level.
pub fn classify_dirs(dir: &Path, skip_hidden: bool, max_depth: u32) -> Result<ClassifiedDirs> {
    let mut projects = Vec::new();
    let mut others = Vec::new();
    let max_depth = match max_depth {
        0 => DEPTH_CEILING,
        n => (n as usize).min(DEPTH_CEILING),
    };

    classify_walk(dir, 1, max_depth, skip_hidden, &mut projects, &mut others)?;

    Ok((projects, others))
}

fn classify_walk(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    skip_hidden: bool,
    projects: &mut Vec<(std::path::PathBuf, u32)>,
    others: &mut Vec<std::path::PathBuf>,
) -> Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();

    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if skip_hidden && name.starts_with('.') {
            continue;
        }
        if SKIP_DIR_NAMES.contains(&name) {
            continue;
        }

        // Symlinks are classified but never descended into, so a link cycle
        // cannot drive the walk.
        let link = path.is_symlink();

        if is_git_repo(&path) {
            projects.push((path, depth as u32));
            continue;
        }

        others.push(path.clone());
        if !link && depth < max_depth {
            classify_walk(&path, depth + 1, max_depth, skip_hidden, projects, others)?;
        }
    }

    Ok(())
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

    /// Build an isolated scratch tree; each test owns its own name so parallel
    /// runs cannot collide.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("projector_test_classify_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn git_repo(dir: &Path) {
        std::fs::create_dir_all(dir.join(".git")).unwrap();
    }

    fn names_at_depth(found: &[(std::path::PathBuf, u32)], depth: u32) -> Vec<String> {
        let mut v: Vec<String> = found
            .iter()
            .filter(|(_, d)| *d == depth)
            .map(|(p, _)| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn test_classify_depth_one_finds_only_direct_children() {
        let root = scratch("depth1");
        git_repo(&root.join("app"));
        std::fs::create_dir_all(root.join("mono").join("inner")).unwrap();
        git_repo(&root.join("mono").join("inner"));

        let (found, others) = classify_dirs(&root, true, 1).unwrap();
        assert_eq!(names_at_depth(&found, 1), vec!["app"]);
        assert!(
            names_at_depth(&found, 2).is_empty(),
            "max_depth=1 must not descend"
        );
        assert!(
            others.iter().any(|p| p.file_name().unwrap() == "mono"),
            "non-project child is still reported"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_depth_two_finds_nested_repo() {
        let root = scratch("depth2");
        git_repo(&root.join("app"));
        git_repo(&root.join("mono").join("inner"));

        let (found, _) = classify_dirs(&root, true, 2).unwrap();
        assert_eq!(names_at_depth(&found, 1), vec!["app"]);
        assert_eq!(names_at_depth(&found, 2), vec!["inner"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_depth_zero_is_unlimited_but_bounded() {
        let root = scratch("depth_unlimited");
        let mut cur = root.clone();
        for _ in 0..40 {
            cur = cur.join("nest");
        }
        git_repo(&cur);

        let (found, _) = classify_dirs(&root, true, 0).unwrap();
        // The deep repo is past the ceiling, so the walk stops without hanging.
        assert!(found.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_does_not_descend_into_a_repo() {
        let root = scratch("no_descend");
        git_repo(&root.join("outer"));
        git_repo(&root.join("outer").join("vendored"));

        let (found, _) = classify_dirs(&root, true, 5).unwrap();
        assert_eq!(names_at_depth(&found, 1), vec!["outer"]);
        assert_eq!(found.len(), 1, "repos inside a repo are not reported");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_skips_node_modules_and_target() {
        let root = scratch("skipdirs");
        git_repo(&root.join("node_modules").join("dep"));
        git_repo(&root.join("target").join("build"));
        git_repo(&root.join("real"));

        let (found, others) = classify_dirs(&root, false, 5).unwrap();
        assert_eq!(names_at_depth(&found, 1), vec!["real"]);
        assert!(others.is_empty(), "skipped dirs are not listed either");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_skip_hidden_controls_descending() {
        let root = scratch("hidden");
        git_repo(&root.join(".secrets").join("repo"));
        git_repo(&root.join("visible"));

        let (skipped, _) = classify_dirs(&root, true, 5).unwrap();
        assert_eq!(names_at_depth(&skipped, 1), vec!["visible"]);
        assert_eq!(skipped.len(), 1);

        let (included, _) = classify_dirs(&root, false, 5).unwrap();
        assert_eq!(names_at_depth(&included, 1), vec!["visible"]);
        assert_eq!(names_at_depth(&included, 2), vec!["repo"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_depth_reported_matches_nesting() {
        let root = scratch("depth_values");
        git_repo(&root.join("a").join("b").join("c"));

        let (found, _) = classify_dirs(&root, true, 3).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, 3);
        assert_eq!(found[0].0, root.join("a").join("b").join("c"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_classify_empty_dir() {
        let root = scratch("empty");
        std::fs::create_dir_all(&root).unwrap();
        let (found, others) = classify_dirs(&root, true, 3).unwrap();
        assert!(found.is_empty());
        assert!(others.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A symlinked directory pointing back at the root would recurse forever if
    /// the walk followed it.
    #[cfg(unix)]
    #[test]
    fn test_classify_does_not_follow_symlink_cycles() {
        let root = scratch("symlink");
        std::fs::create_dir_all(&root).unwrap();
        git_repo(&root.join("real"));
        std::os::unix::fs::symlink(&root, root.join("loopback")).unwrap();

        let (found, others) = classify_dirs(&root, true, 5).unwrap();
        assert_eq!(names_at_depth(&found, 1), vec!["real"]);
        assert_eq!(found.len(), 1, "the cycle yields no extra projects");
        assert!(
            others.iter().any(|p| p.file_name().unwrap() == "loopback"),
            "the symlink itself is still listed as a plain directory"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
