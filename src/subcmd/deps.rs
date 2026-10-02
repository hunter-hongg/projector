use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::color;
use crate::dependencies;
use crate::format::OutputFormat;
use crate::snapshot::SnapshotStore;

pub fn subcmd_deps(
    path: Option<String>,
    shared: bool,
    project: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let format = OutputFormat::parse(format.as_deref(), &[OutputFormat::Json])?;

    let deps = match path {
        Some(p) => {
            let p_path = Path::new(&p);
            if !p_path.exists() {
                anyhow::bail!("Path '{}' does not exist", p);
            }
            dependencies::parse_dependencies(p_path)
        }
        None => {
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

            let mut all_deps = Vec::new();
            for proj in &latest.projects {
                let dir = Path::new(&proj.path);
                if dir.exists() {
                    all_deps.extend(dependencies::parse_dependencies(dir));
                }
            }
            all_deps
        }
    };

    if shared {
        let shared_deps = find_shared(&deps);
        if format.is_json() {
            print_json_shared(&shared_deps, &deps)?;
        } else {
            print_shared(&shared_deps, &deps);
        }
        return Ok(());
    }

    let deps = if let Some(ref proj_name) = project {
        deps.into_iter()
            .filter(|d| {
                Path::new(&d.project_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.contains(proj_name))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        deps
    };

    if format.is_json() {
        print_json_all(&deps)?;
    } else {
        print_all(&deps);
    }

    Ok(())
}

struct SharedDep {
    name: String,
    dep_type: String,
    /// Distinct (deduped) version requirements across the projects using this dep.
    versions: Vec<String>,
    projects: Vec<String>,
}

impl SharedDep {
    /// True when projects pin different version requirements for this dep.
    fn has_conflict(&self) -> bool {
        self.versions.len() > 1
    }
}

fn find_shared(deps: &[dependencies::DependencyEntry]) -> Vec<SharedDep> {
    let mut by_name: HashMap<(String, String), Vec<&dependencies::DependencyEntry>> =
        HashMap::new();
    for d in deps {
        by_name
            .entry((d.name.clone(), d.dep_type.clone()))
            .or_default()
            .push(d);
    }

    let mut result: Vec<SharedDep> = by_name
        .into_iter()
        .filter(|(_, entries)| {
            entries
                .iter()
                .filter_map(|e| {
                    Path::new(&e.project_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                })
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= 2
        })
        .map(|((name, dep_type), entries)| {
            let mut versions: Vec<String> = entries
                .iter()
                .map(|e| e.version_req.trim().to_string())
                .collect();
            versions.sort();
            versions.dedup();
            let mut projects: Vec<String> = entries
                .iter()
                .map(|e| {
                    Path::new(&e.project_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&e.project_path)
                        .to_string()
                })
                .collect();
            projects.sort();
            projects.dedup();
            SharedDep {
                name,
                dep_type,
                versions,
                projects,
            }
        })
        .collect();

    result.sort_by_key(|b| std::cmp::Reverse(b.projects.len()));
    result
}

fn print_shared(shared: &[SharedDep], all: &[dependencies::DependencyEntry]) {
    let project_count = count_projects(all);

    println!();
    println!(
        "  {}",
        color::info(&format!("Dependency Report — {} projects", project_count))
    );
    println!("  ========================================");

    if shared.is_empty() {
        println!("  No shared dependencies across projects.");
        return;
    }

    let conflicts = shared.iter().filter(|s| s.has_conflict()).count();
    if conflicts > 0 {
        println!(
            "  Shared dependencies (used by 2+ projects) — ⚠ {} with version conflicts:",
            conflicts
        );
    } else {
        println!("  Shared dependencies (used by 2+ projects):");
    }
    println!();
    for dep in shared {
        let type_colored = match dep.dep_type.as_str() {
            "rust" => color::green(&dep.dep_type),
            "js" => color::yellow(&dep.dep_type),
            "go" => color::blue(&dep.dep_type),
            "python" => color::cyan(&dep.dep_type),
            _ => color::white(&dep.dep_type),
        };

        let version_string = dep.versions.join(", ");
        let conflict_marker = if dep.has_conflict() {
            color::red(&format!(
                "⚠ {} conflicting requirements",
                dep.versions.len()
            ))
        } else {
            color::green("consistent")
        };
        println!(
            "    {:<16} {:<20} {} {}    used by: {}",
            color::cyan(&dep.name),
            version_string,
            type_colored,
            conflict_marker,
            dep.projects.join(", "),
        );
    }
}

fn print_json_shared(shared: &[SharedDep], all: &[dependencies::DependencyEntry]) -> Result<()> {
    let shared_json: Vec<serde_json::Value> = shared
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "version": s.versions.first().cloned().unwrap_or_default(),
                "versions": s.versions,
                "conflict": s.has_conflict(),
                "type": s.dep_type,
                "projects": s.projects
            })
        })
        .collect();

    let total_projects = count_projects(all);
    let unique_deps_count = count_unique(all);
    let conflicts_count = shared.iter().filter(|s| s.has_conflict()).count();

    let output = serde_json::json!({
        "shared": shared_json,
        "total_projects": total_projects,
        "total_deps": all.len(),
        "unique_deps": unique_deps_count,
        "shared_dep_count": shared.len(),
        "conflicts_count": conflicts_count,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_all(deps: &[dependencies::DependencyEntry]) {
    let project_count = count_projects(deps);

    println!();
    println!(
        "  {}",
        color::info(&format!("Dependency Report — {} projects", project_count))
    );
    println!("  ========================================");

    let mut by_project: HashMap<String, Vec<&dependencies::DependencyEntry>> = HashMap::new();
    for d in deps {
        by_project
            .entry(d.project_path.clone())
            .or_default()
            .push(d);
    }

    for (path, project_deps) in &by_project {
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let dev_count = project_deps.iter().filter(|d| d.is_dev).count();
        let total = project_deps.len();

        let dep_type = project_deps
            .first()
            .map(|d| d.dep_type.as_str())
            .unwrap_or("unknown");
        let type_colored = match dep_type {
            "rust" => color::green("Rust"),
            "js" => color::yellow("JS"),
            "go" => color::blue("Go"),
            "python" => color::cyan("Python"),
            _ => color::white("Unknown"),
        };

        if dev_count > 0 {
            println!(
                "    {:<20} {}    {} deps ({} dev)",
                color::cyan(name),
                type_colored,
                total,
                dev_count
            );
        } else {
            println!(
                "    {:<20} {}    {} deps",
                color::cyan(name),
                type_colored,
                total
            );
        }
    }
}

fn print_json_all(deps: &[dependencies::DependencyEntry]) -> Result<()> {
    let total_projects = count_projects(deps);
    let unique_deps_count = count_unique(deps);

    let mut by_project: HashMap<String, Vec<&dependencies::DependencyEntry>> = HashMap::new();
    for d in deps {
        by_project
            .entry(d.project_path.clone())
            .or_default()
            .push(d);
    }

    let projects_json: Vec<serde_json::Value> = by_project
        .into_iter()
        .map(|(path, project_deps)| {
            let deps_json: Vec<serde_json::Value> = project_deps
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "name": d.name,
                        "version": d.version_req,
                        "type": d.dep_type,
                        "is_dev": d.is_dev
                    })
                })
                .collect();
            let dev_count = project_deps.iter().filter(|d| d.is_dev).count();
            serde_json::json!({
                "path": path,
                "total_deps": project_deps.len(),
                "dev_deps": dev_count,
                "deps": deps_json
            })
        })
        .collect();

    let output = serde_json::json!({
        "projects": projects_json,
        "total_projects": total_projects,
        "total_deps": deps.len(),
        "unique_deps": unique_deps_count,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn count_projects(deps: &[dependencies::DependencyEntry]) -> usize {
    let mut paths: Vec<&str> = deps.iter().map(|d| d.project_path.as_str()).collect();
    paths.sort();
    paths.dedup();
    paths.len()
}

fn count_unique(deps: &[dependencies::DependencyEntry]) -> usize {
    let mut names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    names.sort();
    names.dedup();
    names.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        name: &str,
        version: &str,
        proj: &str,
        dep_type: &str,
    ) -> dependencies::DependencyEntry {
        dependencies::DependencyEntry {
            name: name.to_string(),
            version_req: version.to_string(),
            project_path: format!("/proj/{}_root/{}", dep_type, proj),
            dep_type: dep_type.to_string(),
            is_dev: false,
        }
    }

    #[test]
    fn test_find_shared_groups_by_name_and_type() {
        let deps = vec![
            entry("serde", "1.0", "a", "rust"),
            entry("serde", "1.0", "b", "rust"),
            // Same crate name in another ecosystem must not merge.
            entry("serde", "3.0", "a", "js"),
        ];
        let shared = find_shared(&deps);
        assert_eq!(shared.len(), 1);
        assert_eq!(shared[0].name, "serde");
        assert_eq!(shared[0].dep_type, "rust");
        assert_eq!(shared[0].projects, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn test_find_shared_reports_version_conflict() {
        let deps = vec![
            entry("serde", "1.0", "a", "rust"),
            entry("serde", "1.0", "b", "rust"),
            entry("serde", "^2.0", "c", "rust"),
        ];
        let shared = find_shared(&deps);
        assert_eq!(shared.len(), 1);
        assert!(shared[0].has_conflict());
        assert_eq!(
            shared[0].versions,
            vec!["1.0".to_string(), "^2.0".to_string()]
        );
    }

    #[test]
    fn test_find_shared_matching_versions_are_not_a_conflict() {
        let deps = vec![
            entry("serde", "1.0", "a", "rust"),
            entry("serde", "1.0", "b", "rust"),
        ];
        let shared = find_shared(&deps);
        assert_eq!(shared.len(), 1);
        assert!(!shared[0].has_conflict());
    }

    #[test]
    fn test_find_shared_requires_two_distinct_projects() {
        // Both entries come from the same project: not shared.
        let mut a1 = entry("serde", "1.0", "a", "rust");
        let mut a2 = entry("serde", "1.0", "a", "rust");
        a1.is_dev = false;
        a2.is_dev = true;
        let shared = find_shared(&[a1, a2]);
        assert!(shared.is_empty());
    }

    #[test]
    fn test_find_shared_wildcard_treated_as_distinct() {
        let deps = vec![
            entry("serde", "1.0", "a", "rust"),
            entry("serde", "*", "b", "rust"),
        ];
        let shared = find_shared(&deps);
        assert_eq!(shared.len(), 1);
        assert!(shared[0].has_conflict());
    }

    #[test]
    fn test_shared_sorted_by_project_count_desc() {
        let deps = vec![
            entry("serde", "1.0", "a", "rust"),
            entry("serde", "1.0", "b", "rust"),
            entry("serde", "1.0", "c", "rust"),
            entry("tokio", "1.0", "a", "rust"),
            entry("tokio", "1.0", "b", "rust"),
        ];
        let shared = find_shared(&deps);
        assert_eq!(shared[0].name, "serde");
        assert_eq!(shared[1].name, "tokio");
    }
}
