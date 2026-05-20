use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::analyzer;
use crate::color;

pub fn subcmd_list(dir: Option<String>) -> Result<()> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let dir_path = Path::new(&dir);

    let (projects, others) = analyzer::classify_dirs(dir_path, false)?;

    println!("{}", color::info("listing directories..."));
    println!();
    println!("{}", color::green("Projects:"));

    for p in &projects {
        let project_type = analyzer::ProjectType::detect(p)?;
        let type_str = project_type.as_str();
        let display_type = if type_str == "Unknown" {
            color::red("Unknown project")
        } else {
            color::green(&format!("{} project", type_str))
        };

        let metadata = fs::metadata(p)?;
        let modified_time = metadata.modified()?;
        let dt: chrono::DateTime<chrono::Local> = modified_time.into();
        let last_modified_str = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        let too_long = (chrono::Local::now() - dt).num_days() > 30;
        let display_last = if too_long {
            color::red(&last_modified_str)
        } else {
            color::green(&last_modified_str)
        };

        println!(
            "project {}: {}, last modified: {}",
            color::cyan(&p.to_string_lossy()),
            display_type,
            display_last,
        );
    }

    println!();
    println!("{}", color::green("Other directories:"));
    for o in &others {
        println!("dir {}: ", color::red(&o.to_string_lossy()));
    }

    Ok(())
}
