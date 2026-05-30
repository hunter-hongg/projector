use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::analyzer;
use crate::color;
use crate::snapshot::SnapshotStore;

pub fn subcmd_deps(
    path: Option<String>,
    shared: bool,
    project: Option<String>,
    format: Option<String>,
) -> Result<()> {
    let format = format.unwrap_or_default();
    if !format.is_empty() && format != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", format);
    }

    let deps = match path {
        Some(p) => {
            let p_path = Path::new(&p);
            if !p_path.exists() {
                anyhow::bail!("Path '{}' does not exist", p);
            }
            analyzer::parse_dependencies(p_path)
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
                    all_deps.extend(analyzer::parse_dependencies(dir));
                }
            }
            all_deps
        }
    };

    if shared {
        let shared_deps = find_shared(&deps);
        if format == "json" {
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

    if format == "json" {
            print_json_all(&deps)?;
        } else {
            print_all(&deps);
        }

    Ok(())
}

struct SharedDep {
    name: String,
    version: String,
    dep_type: String,
    projects: Vec<String>,
}

fn find_shared(deps: &[analyzer::DependencyEntry]) -> Vec<SharedDep> {
    let mut by_name: HashMap<&str, Vec<&analyzer::DependencyEntry>> = HashMap::new();
    for d in deps {
        by_name.entry(&d.name).or_default().push(d);
    }

    let mut result: Vec<SharedDep> = by_name
        .into_iter()
        .filter(|(_, entries)| entries.len() >= 2)
        .map(|(name, entries)| {
            let first = entries[0];
            let version = first.version_req.clone();
            let dep_type = first.dep_type.clone();
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
                name: name.to_string(),
                version,
                dep_type,
                projects,
            }
        })
        .collect();

    result.sort_by_key(|b| std::cmp::Reverse(b.projects.len()));
    result
}

fn print_shared(shared: &[SharedDep], all: &[analyzer::DependencyEntry]) {
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

    println!("  Shared dependencies (used by 2+ projects):");
    println!();
    for dep in shared {
        let type_colored = match dep.dep_type.as_str() {
            "rust" => color::green(&dep.dep_type),
            "js" => color::yellow(&dep.dep_type),
            "go" => color::blue(&dep.dep_type),
            "python" => color::cyan(&dep.dep_type),
            _ => color::white(&dep.dep_type),
        };
        println!(
            "    {:<16} {:<8} {}    used by: {}",
            color::cyan(&dep.name),
            dep.version,
            type_colored,
            dep.projects.join(", "),
        );
    }
}

fn print_json_shared(shared: &[SharedDep], all: &[analyzer::DependencyEntry]) -> Result<()> {
    let shared_json: Vec<serde_json::Value> = shared
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "version": s.version,
                "type": s.dep_type,
                "projects": s.projects
            })
        })
        .collect();

    let total_projects = count_projects(all);
    let unique_deps_count = count_unique(all);

    let output = serde_json::json!({
        "shared": shared_json,
        "total_projects": total_projects,
        "total_deps": all.len(),
        "unique_deps": unique_deps_count,
        "shared_dep_count": shared.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_all(deps: &[analyzer::DependencyEntry]) {
    let project_count = count_projects(deps);

    println!();
    println!(
        "  {}",
        color::info(&format!("Dependency Report — {} projects", project_count))
    );
    println!("  ========================================");

    let mut by_project: HashMap<String, Vec<&analyzer::DependencyEntry>> = HashMap::new();
    for d in deps {
        by_project.entry(d.project_path.clone()).or_default().push(d);
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

fn print_json_all(deps: &[analyzer::DependencyEntry]) -> Result<()> {
    let total_projects = count_projects(deps);
    let unique_deps_count = count_unique(deps);

    let mut by_project: HashMap<String, Vec<&analyzer::DependencyEntry>> = HashMap::new();
    for d in deps {
        by_project.entry(d.project_path.clone()).or_default().push(d);
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

fn count_projects(deps: &[analyzer::DependencyEntry]) -> usize {
    let mut paths: Vec<&str> = deps.iter().map(|d| d.project_path.as_str()).collect();
    paths.sort();
    paths.dedup();
    paths.len()
}

fn count_unique(deps: &[analyzer::DependencyEntry]) -> usize {
    let mut names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    names.sort();
    names.dedup();
    names.len()
}
