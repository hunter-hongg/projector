use std::collections::HashMap;
use std::fs;

use anyhow::Result;

use crate::color;

fn listdir(dir: &String) -> Result<Vec<String>> {
    let entries = fs::read_dir(dir)?;
    let mut dirs = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path().to_string_lossy().to_string());
        }
    }
    Ok(dirs)
}

fn has_file(dir: &String, file: &str) -> Result<bool> {
    let _dirs = fs::read_dir(dir)?;
    let mut files = vec![];
    for entry in _dirs {
        let entry = entry?;
        files.push(entry.path().to_string_lossy().to_string());
    }
    Ok(files
        .iter()
        .any(|x| x.to_lowercase().ends_with(&file.to_lowercase())))
}

fn detect_by_extensions(dir: &str) -> String {
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
        (rust, "Rust"),
        (js, "JavaScript/TypeScript"),
        (go, "Go"),
        (py, "Python"),
        (java_kt, "Java/Kotlin"),
        (cpp, "C/C++"),
        (ocaml, "OCaml"),
        (dart, "Dart"),
    ];

    candidates
        .into_iter()
        .max_by_key(|(count, _)| *count)
        .filter(|(count, _)| *count > 0)
        .map(|(_, pt)| pt.to_string())
        .unwrap_or("Unknown".to_string())
}

fn scan_dir_for_extensions(dir: &str, counts: &mut HashMap<String, u32>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
                scan_dir_for_extensions(&path.to_string_lossy(), counts);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    *counts.entry(ext.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
}

pub fn subcmd_list(dir: Option<String>) -> Result<()> {
    let dir = dir.unwrap_or(".".to_string());
    let dirs = listdir(&dir)?;
    let mut has_git = vec![];
    let mut no_git = vec![];
    for i in dirs.clone() {
        let d_inner = listdir(&i)?;
        if d_inner.iter().any(|x| x.ends_with(".git")) {
            has_git.push(i);
        } else {
            no_git.push(i);
        }
    }
    println!("{}", color::info("listing directories..."));
    println!();
    println!("{}", color::green("Projects:"));
    for i in has_git {
        let project_type = if has_file(&i, "CMakeLists.txt")? {
            "C/C++".to_string()
        } else if has_file(&i, "Cargo.toml")? {
            "Rust".to_string()
        } else if has_file(&i, "package.json")? {
            "JavaScript/TypeScript".to_string()
        } else if has_file(&i, "dune-project")? {
            "OCaml".to_string()
        } else if has_file(&i, "build.gradle")? || has_file(&i, "pom.xml")? {
            "Java/Kotlin".to_string()
        } else if has_file(&i, "go.mod")? {
            "Go".to_string()
        } else if has_file(&i, "pubspec.yaml")? {
            "Dart".to_string()
        } else {
            detect_by_extensions(&i)
        };
        let prettier_type = if project_type == "Unknown" {
            &color::red("Unknown project")
        } else {
            &color::green(format!("{} project", project_type).as_str())
        };
        let last_modified = fs::metadata(&i)?;
        let last_modified = last_modified.modified()?;
        let last_modified = chrono::DateTime::<chrono::Local>::from(last_modified);
        let last_modified_str = last_modified.format("%Y-%m-%d %H:%M:%S").to_string();
        let too_long = (chrono::Local::now() - last_modified).num_days() > 30;
        let prettier_last_modified = if too_long {
            &color::red(last_modified_str.as_str())
        } else {
            &color::green(last_modified_str.as_str())
        };
        println!(
            "project {}: {}, last modified: {}",
            color::cyan(&i),
            (prettier_type),
            prettier_last_modified
        );
    }
    println!();
    println!("{}", color::green("Other directories:"));
    for i in no_git {
        println!("dir {}: ", color::red(&i))
    }
    Ok(())
}
