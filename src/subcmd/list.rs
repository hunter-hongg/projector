use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::analyzer;
use crate::color;
use crate::tags::TagsIndex;

pub fn subcmd_list(dir: Option<String>, tag: Option<String>) -> Result<()> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    let dir_path = Path::new(&dir);

    let (projects, others) = analyzer::classify_dirs(dir_path, false)?;

    let tags_index = TagsIndex::load()?;

    let tag_display = tag.clone();
    let filtered: Vec<_> = if let Some(ref tag_name) = tag {
        projects
            .into_iter()
            .filter(|p| tags_index.has_tag(&p.to_string_lossy(), tag_name))
            .collect()
    } else {
        projects
    };

    if let Some(ref tag_name) = tag_display {
        println!(
            "{}",
            color::info(&format!(
                "listing directories with tag '{}'...",
                tag_name
            ))
        );
    } else {
        println!("{}", color::info("listing directories..."));
    }
    println!();
    println!("{}", color::green("Projects:"));

    for p in &filtered {
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

        let tags = tags_index.tags_for_path(&p.to_string_lossy());
        let tag_str = if tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tags.join(", "))
        };

        println!(
            "project {}: {}, last modified: {}{}",
            color::cyan(&p.to_string_lossy()),
            display_type,
            display_last,
            if tag_str.is_empty() {
                String::new()
            } else {
                color::yellow(&tag_str)
            },
        );
    }

    if let Some(ref tag_name) = tag_display
        && filtered.is_empty()
    {
        println!(
            "  {}",
            color::yellow(&format!(
                "No projects with tag '{}' found.",
                tag_name
            ))
        );
    }

    println!();
    println!("{}", color::green("Other directories:"));
    for o in &others {
        println!("dir {}: ", color::red(&o.to_string_lossy()));
    }

    Ok(())
}
