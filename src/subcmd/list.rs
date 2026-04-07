use std::{fs};

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
    println!("project {}: ", color::cyan(&i))
  }
  println!("");
  println!("{}", color::green("Other directories:"));
  for i in no_git {
    println!("dir {}: ", color::red(&i))
  }
  Ok(())
}
