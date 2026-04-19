use std::{fmt::Debug, fs};

use anyhow::Result;

use crate::color;

fn listdir(dir: &String) -> Result<Vec<String>> {
  let entries = fs::read_dir(&dir)?;
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
  Ok(files.iter().any(|x| 
    x.to_lowercase().ends_with(&file.to_lowercase()))
  )
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
  println!("");
  println!("{}", color::green("Projects:"));
  for i in has_git {
    let project_type = 
    if has_file(&i, "CMakeLists.txt")? {
      "C/C++"
    } else if has_file(&i, "Cargo.toml")? {
      "Rust"
    } else if has_file(&i, "package.json")? {
      "JavaScript/TypeScript"
    } else if has_file(&i, "dune-project")? {
      "OCaml"
    } else if has_file(&i, "build.gradle")? || has_file(&i, "pom.xml")? {
      "Java/Kotlin"
    } else if has_file(&i, "go.mod")? {
      "Go"
    } else if has_file(&i, "pubspec.yaml")? {
      "Dart"
    } else {
      "Unknown"
    };
    let prettier_type = if project_type == "Unknown" {
      &color::red("Unknown project")
    } else {
      &color::green(format!("{} project", project_type).as_str())
    };
    let last_modified = fs::metadata(&i)?;
    let last_modified = last_modified.modified()?;
    let last_modified = 
      chrono::DateTime::<chrono::Local>::from(last_modified);
    let last_modified_str = last_modified.format("%Y-%m-%d %H:%M:%S").to_string();
    let too_long = 
      (chrono::Local::now() - last_modified).num_days() > 30;
    let prettier_last_modified = if too_long {
      &color::red(last_modified_str.as_str())
    } else {
      &color::green(last_modified_str.as_str())
    };
    println!("project {}: {}, last modified: {}",
      color::cyan(&i), (
        prettier_type
      ), prettier_last_modified);
  }
  println!("");
  println!("{}", color::green("Other directories:"));
  for i in no_git {
    println!("dir {}: ", color::red(&i))
  }
  Ok(())
}
