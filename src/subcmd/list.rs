use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::color;
use crate::detect;
use crate::format::OutputFormat;
use crate::tags::TagsIndex;

pub fn subcmd_list(
    dir: Option<String>,
    tag: Option<String>,
    type_filter: Option<String>,
    depth: Option<u32>,
    format: Option<String>,
) -> Result<()> {
    let fmt = OutputFormat::parse(format.as_deref(), &[OutputFormat::Json])?;

    let dir = dir.unwrap_or_else(|| ".".to_string());
    let dir_path = Path::new(&dir);
    let max_depth = crate::config::Config::load()?.scan.max_depth;

    let (projects, others) = detect::classify_dirs(dir_path, false, max_depth)?;

    let tags_index = TagsIndex::load()?;

    let tag_display = tag.clone();
    let filtered: Vec<_> = projects
        .into_iter()
        .filter(|(p, d)| {
            matches_filters(
                p,
                *d,
                tag.as_deref(),
                type_filter.as_deref(),
                depth,
                &tags_index,
            )
        })
        .collect();

    if fmt.is_json() {
        let json: Vec<serde_json::Value> = filtered
            .iter()
            .map(|(p, d)| {
                let project_type = detect::ProjectType::detect(p)
                    .map(|pt| pt.as_str().to_string())
                    .unwrap_or_else(|_| "Unknown".to_string());
                let tags = tags_index.tags_for_path(&p.to_string_lossy());
                serde_json::json!({
                    "path": p,
                    "project_type": project_type,
                    "depth": d,
                    "last_modified": fs::metadata(p)
                        .and_then(|m| m.modified())
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Local> = t.into();
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_default(),
                    "tags": tags,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    if let Some(ref tag_name) = tag_display {
        println!(
            "{}",
            color::info(&format!("listing directories with tag '{}'...", tag_name))
        );
    } else {
        println!("{}", color::info("listing directories..."));
    }
    println!();
    println!("{}", color::green("Projects:"));

    for (p, depth) in &filtered {
        let project_type = detect::ProjectType::detect(p)?;
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
            "project {}: {}, last modified: {}{}{}",
            color::cyan(&p.to_string_lossy()),
            display_type,
            display_last,
            color::blue(&format!(", depth {}", depth)),
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
            color::yellow(&format!("No projects with tag '{}' found.", tag_name))
        );
    }

    println!();
    println!("{}", color::green("Other directories:"));
    for o in &others {
        println!("dir {}: ", color::red(&o.to_string_lossy()));
    }

    Ok(())
}

/// Decide whether a discovered project at `depth` passes the `list` filters.
///
/// All filters are optional and AND together: a missing filter never excludes.
/// `--type` matches any project type whose name contains the substring
/// (same convention as `rank --type`); `--depth` requires an exact match.
fn matches_filters(
    path: &Path,
    depth: u32,
    tag: Option<&str>,
    type_filter: Option<&str>,
    depth_filter: Option<u32>,
    tags: &TagsIndex,
) -> bool {
    if let Some(tag_name) = tag
        && !tags.has_tag(&path.to_string_lossy(), tag_name)
    {
        return false;
    }
    if let Some(t) = type_filter
        && !detect::ProjectType::detect(path)
            .map(|pt| pt.as_str().contains(t))
            .unwrap_or(false)
    {
        return false;
    }
    if let Some(d) = depth_filter
        && depth != d
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("projector_list_filter_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn git_repo(dir: &std::path::Path) {
        std::fs::create_dir_all(dir.join(".git")).unwrap();
    }

    #[test]
    fn test_matches_filters_type_substring() {
        let root = scratch("type");
        let app = root.join("app");
        git_repo(&app);
        std::fs::write(app.join("Cargo.toml"), "").unwrap();
        let tags = TagsIndex::load().unwrap();

        let (found, _) = detect::classify_dirs(&root, true, 1).unwrap();
        assert!(!found.is_empty(), "app must be discovered");
        let mut matched = false;
        for (path, depth) in &found {
            if matches_filters(path, *depth, None, Some("Rust"), None, &tags) {
                matched = true;
            }
        }
        assert!(matched, "Rust project must pass --type Rust");
        // A non-matching type filter must exclude it.
        assert!(
            found
                .iter()
                .all(|(p, d)| !matches_filters(p, *d, None, Some("Go"), None, &tags)),
            "--type Go must not match a Rust project"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_matches_filters_depth_exact() {
        let root = scratch("depth");
        git_repo(&root.join("inner").join("deep"));
        std::fs::create_dir_all(root.join("inner")).unwrap();
        let tags = TagsIndex::load().unwrap();

        let (found, _) = detect::classify_dirs(&root, true, 3).unwrap();
        let by_depth_2: Vec<_> = found
            .iter()
            .filter(|(p, d)| matches_filters(p, *d, None, None, Some(2), &tags))
            .collect();
        assert_eq!(by_depth_2.len(), 1);
        assert_eq!(by_depth_2[0].1, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_matches_filters_all_filters_none_excludes() {
        let root = scratch("none");
        git_repo(&root.join("app"));
        let tags = TagsIndex::load().unwrap();
        let (found, _) = detect::classify_dirs(&root, true, 1).unwrap();
        for (path, depth) in &found {
            assert!(matches_filters(path, *depth, None, None, None, &tags));
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
