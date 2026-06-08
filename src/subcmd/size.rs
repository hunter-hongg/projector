use std::path::Path;
use anyhow::Result;

use crate::analyzer;
use crate::color;
use crate::snapshot::SnapshotStore;

pub fn subcmd_size(
    path: Option<String>,
    top: Option<usize>,
    deep: bool,
    format: Option<String>,
) -> Result<()> {
    let fmt = format.unwrap_or_default();
    if !fmt.is_empty() && fmt != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", fmt);
    }

    if let Some(p) = path {
        let dir = Path::new(&p);
        if !dir.exists() {
            anyhow::bail!("Path not found: {}", p);
        }
        if deep {
            print_deep_breakdown(dir, &fmt)?;
        } else {
            let size = analyzer::calc_dir_size(dir, true);
            if fmt == "json" {
                let json = serde_json::json!({
                    "path": p,
                    "size": size,
                    "size_human": analyzer::human_size(size),
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
            } else {
                println!("  {}: {}", color::cyan(&p), analyzer::human_size(size));
            }
        }
        return Ok(());
    }

    let latest = match SnapshotStore::load_latest()? {
        Some(s) => s,
        None => {
            println!("{}", color::error("No snapshots found. Run `projector scan` first."));
            return Ok(());
        }
    };

    let mut entries: Vec<(String, u64)> = Vec::new();
    for proj in &latest.projects {
        let dir = Path::new(&proj.path);
        if dir.exists() {
            let size = analyzer::calc_dir_size(dir, true);
            entries.push((proj.path.clone(), size));
        }
    }

    entries.sort_by_key(|(_, s)| std::cmp::Reverse(*s));

    if let Some(n) = top {
        entries.truncate(n);
    }

    if fmt == "json" {
        let json: Vec<serde_json::Value> = entries.iter().map(|(path, size)| {
            serde_json::json!({
                "path": path,
                "size": size,
                "size_human": analyzer::human_size(*size),
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!();
        println!("  {}", color::info("Project disk usage"));
        println!();
        let max_name = entries.iter().map(|(p, _)| p.split('/').next_back().unwrap_or(p).len()).max().unwrap_or(20).min(40);
        for (path, size) in &entries {
            let name = path.split('/').next_back().unwrap_or(path);
            println!("  {:max_name$}  {}", color::cyan(name), analyzer::human_size(*size), max_name = max_name);
        }
        println!();
    }

    Ok(())
}

fn print_deep_breakdown(dir: &Path, fmt: &str) -> Result<()> {
    let total = analyzer::calc_dir_size(dir, false);
    let source = dir_size_by_extensions(dir, &["rs", "js", "ts", "jsx", "tsx", "go", "py", "java", "kt", "kts", "c", "h", "cpp", "hpp", "cc", "cxx", "ml", "mli", "dart", "toml", "json", "yaml", "yml", "md", "css", "html", "sh", "bash", "zsh", "fish"]);
    let deps = dir_size_by_name(dir, &["node_modules", "target"]);
    let git = dir_size_by_name(dir, &[".git"]);
    let other = total.saturating_sub(source + deps + git);

    if fmt == "json" {
        let json = serde_json::json!({
            "path": dir.to_string_lossy(),
            "total": total,
            "total_human": analyzer::human_size(total),
            "breakdown": {
                "source": source,
                "deps": deps,
                "git": git,
                "other": other,
            }
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("  Deep breakdown for {}", color::cyan(&dir.to_string_lossy()));
        println!("    Total: {}", analyzer::human_size(total));
        println!("    Source code:  {}", analyzer::human_size(source));
        println!("    Dependencies: {}", analyzer::human_size(deps));
        println!("    Git objects:  {}", analyzer::human_size(git));
        println!("    Other:        {}", analyzer::human_size(other));
    }

    Ok(())
}

fn dir_size_by_extensions(dir: &Path, exts: &[&str]) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') && path.is_dir() {
                    continue;
                }
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if exts.contains(&ext) {
                            if let Ok(meta) = std::fs::metadata(&path) {
                                total += meta.len();
                            }
                        }
                    }
                }
            }
        }
    }
    total
}

fn dir_size_by_name(dir: &Path, names: &[&str]) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if names.contains(&name) && path.is_dir() {
                total += analyzer::calc_dir_size(&path, false);
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcmd_size_invalid_format() {
        let result = subcmd_size(None, None, false, Some("xml".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported format"));
    }

    #[test]
    fn test_dir_size_by_extensions_empty() {
        let dir = std::env::temp_dir().join("projector_test_ext_empty");
        let _ = std::fs::create_dir_all(&dir);
        let size = dir_size_by_extensions(&dir, &["rs"]);
        assert_eq!(size, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dir_size_by_name_found() {
        let dir = std::env::temp_dir().join("projector_test_name");
        let _ = std::fs::create_dir_all(dir.join("node_modules"));
        std::fs::write(dir.join("node_modules").join("pkg.js"), "abc").unwrap();
        let size = dir_size_by_name(&dir, &["node_modules"]);
        assert!(size > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dir_size_by_name_not_found() {
        let dir = std::env::temp_dir().join("projector_test_name_miss");
        let _ = std::fs::create_dir_all(&dir);
        let size = dir_size_by_name(&dir, &["node_modules"]);
        assert_eq!(size, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deep_breakdown_has_source() {
        let dir = std::env::temp_dir().join("projector_test_deep");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        let source = dir_size_by_extensions(&dir, &["rs"]);
        assert!(source > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
