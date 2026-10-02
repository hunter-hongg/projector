use std::path::Path;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DependencyEntry {
    pub name: String,
    pub version_req: String,
    pub project_path: String,
    pub dep_type: String,
    pub is_dev: bool,
}

pub fn parse_dependencies(dir: &Path) -> Vec<DependencyEntry> {
    let mut deps = Vec::new();
    let path_str = dir.to_string_lossy().to_string();

    let cargo_path = dir.join("Cargo.toml");
    if cargo_path.exists()
        && let Ok(d) = parse_cargo_deps(&cargo_path, &path_str)
    {
        deps.extend(d);
    }

    let package_json = dir.join("package.json");
    if package_json.exists()
        && let Ok(d) = parse_package_json_deps(&package_json, &path_str)
    {
        deps.extend(d);
    }

    let go_mod = dir.join("go.mod");
    if go_mod.exists()
        && let Ok(d) = parse_go_mod_deps(&go_mod, &path_str)
    {
        deps.extend(d);
    }

    let pyproject = dir.join("pyproject.toml");
    if pyproject.exists()
        && let Ok(d) = parse_pyproject_deps(&pyproject, &path_str)
    {
        deps.extend(d);
    }

    let requirements = dir.join("requirements.txt");
    if requirements.exists()
        && let Ok(d) = parse_requirements_txt(&requirements, &path_str)
    {
        deps.extend(d);
    }

    deps
}

fn parse_cargo_deps(path: &Path, project_path: &str) -> Result<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(path)?;
    let cargo: toml::Value = content.parse()?;
    let mut deps = Vec::new();

    if let Some(table) = cargo.as_table() {
        if let Some(deps_table) = table.get("dependencies").and_then(|v| v.as_table()) {
            for (name, val) in deps_table {
                let version = match val {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };
                deps.push(DependencyEntry {
                    name: name.clone(),
                    version_req: version,
                    project_path: project_path.to_string(),
                    dep_type: "rust".to_string(),
                    is_dev: false,
                });
            }
        }
        if let Some(deps_table) = table.get("dev-dependencies").and_then(|v| v.as_table()) {
            for (name, val) in deps_table {
                let version = match val {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };
                deps.push(DependencyEntry {
                    name: name.clone(),
                    version_req: version,
                    project_path: project_path.to_string(),
                    dep_type: "rust".to_string(),
                    is_dev: true,
                });
            }
        }
    }

    Ok(deps)
}

fn parse_package_json_deps(path: &Path, project_path: &str) -> Result<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    let mut deps = Vec::new();

    if let Some(deps_map) = json.get("dependencies").and_then(|v| v.as_object()) {
        for (name, val) in deps_map {
            let version = val.as_str().unwrap_or("*").to_string();
            deps.push(DependencyEntry {
                name: name.clone(),
                version_req: version,
                project_path: project_path.to_string(),
                dep_type: "js".to_string(),
                is_dev: false,
            });
        }
    }
    if let Some(deps_map) = json.get("devDependencies").and_then(|v| v.as_object()) {
        for (name, val) in deps_map {
            let version = val.as_str().unwrap_or("*").to_string();
            deps.push(DependencyEntry {
                name: name.clone(),
                version_req: version,
                project_path: project_path.to_string(),
                dep_type: "js".to_string(),
                is_dev: true,
            });
        }
    }

    Ok(deps)
}

fn parse_go_mod_deps(path: &Path, project_path: &str) -> Result<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(path)?;
    let mut deps = Vec::new();
    let mut in_require = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("require (") {
            in_require = true;
            continue;
        }
        if in_require && trimmed == ")" {
            in_require = false;
            continue;
        }
        if in_require {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(DependencyEntry {
                    name: parts[0].to_string(),
                    version_req: parts[1].to_string(),
                    project_path: project_path.to_string(),
                    dep_type: "go".to_string(),
                    is_dev: false,
                });
            }
        }
        if !in_require && trimmed.starts_with("require ") && !trimmed.contains('(') {
            let rest = trimmed.trim_start_matches("require ");
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.push(DependencyEntry {
                    name: parts[0].to_string(),
                    version_req: parts[1].to_string(),
                    project_path: project_path.to_string(),
                    dep_type: "go".to_string(),
                    is_dev: false,
                });
            }
        }
    }

    Ok(deps)
}

fn parse_pyproject_deps(path: &Path, project_path: &str) -> Result<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(path)?;
    let pyproject: toml::Value = content.parse()?;
    let mut deps = Vec::new();

    if let Some(table) = pyproject.as_table() {
        if let Some(deps_arr) = table
            .get("project")
            .and_then(|v| v.get("dependencies"))
            .and_then(|v| v.as_array())
        {
            for item in deps_arr {
                if let Some(s) = item.as_str() {
                    let (name, version) = parse_python_dep_spec(s);
                    deps.push(DependencyEntry {
                        name,
                        version_req: version,
                        project_path: project_path.to_string(),
                        dep_type: "python".to_string(),
                        is_dev: false,
                    });
                }
            }
        }
        if let Some(poetry) = table.get("tool").and_then(|v| v.get("poetry")) {
            if let Some(deps_map) = poetry.get("dependencies").and_then(|v| v.as_table()) {
                for (name, val) in deps_map {
                    if name == "python" {
                        continue;
                    }
                    let version = match val {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*")
                            .to_string(),
                        _ => "*".to_string(),
                    };
                    deps.push(DependencyEntry {
                        name: name.clone(),
                        version_req: version,
                        project_path: project_path.to_string(),
                        dep_type: "python".to_string(),
                        is_dev: false,
                    });
                }
            }
            if let Some(deps_map) = poetry.get("dev-dependencies").and_then(|v| v.as_table()) {
                for (name, val) in deps_map {
                    let version = match val {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Table(t) => t
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*")
                            .to_string(),
                        _ => "*".to_string(),
                    };
                    deps.push(DependencyEntry {
                        name: name.clone(),
                        version_req: version,
                        project_path: project_path.to_string(),
                        dep_type: "python".to_string(),
                        is_dev: true,
                    });
                }
            }
        }
    }

    Ok(deps)
}

fn parse_requirements_txt(path: &Path, project_path: &str) -> Result<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(path)?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("-r ") {
            continue;
        }
        let (name, version) = parse_python_dep_spec(trimmed);
        deps.push(DependencyEntry {
            name,
            version_req: version,
            project_path: project_path.to_string(),
            dep_type: "python".to_string(),
            is_dev: false,
        });
    }

    Ok(deps)
}

fn parse_python_dep_spec(s: &str) -> (String, String) {
    let s = s.trim();
    let extras_end = s.find('[').unwrap_or(s.len());
    let base = &s[..extras_end];
    for op in &[">=", "<=", "!=", "==", "~=", ">", "<"] {
        if let Some(pos) = base.find(op) {
            let name = base[..pos].trim().to_string();
            let version = base[pos..].trim().to_string();
            return (name, version);
        }
    }
    (base.to_string(), "*".to_string())
}
