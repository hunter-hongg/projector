use anyhow::Result;

use crate::color;
use crate::snapshot::SnapshotStore;
use crate::tags::TagsIndex;

pub fn subcmd_search(
    query: String,
    tag: Option<String>,
    format: Option<String>,
) -> Result<()> {
    if query.trim().is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }

    let format = format.unwrap_or_default();
    if !format.is_empty() && format != "json" {
        anyhow::bail!("Unsupported format: '{}'. Use 'json'.", format);
    }

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

    let tags_index = TagsIndex::load()?;
    let query_lower = query.to_lowercase();

    let mut results: Vec<(&crate::snapshot::ProjectSnapshot, Vec<String>)> = Vec::new();

    for proj in &latest.projects {
        let name = proj.path.split('/').next_back().unwrap_or(&proj.path);
        let project_tags = tags_index.tags_for_path(&proj.path);

        if let Some(ref tag_filter) = tag
            && !project_tags.iter().any(|t| t == tag_filter)
        {
            continue;
        }

        let matches = name.to_lowercase().contains(&query_lower)
            || proj.path.to_lowercase().contains(&query_lower)
            || proj.project_type.to_lowercase().contains(&query_lower)
            || project_tags.iter().any(|t| t.to_lowercase().contains(&query_lower));

        if matches {
            results.push((proj, project_tags));
        }
    }

    if format == "json" {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|(p, tags)| {
                serde_json::json!({
                    "path": p.path,
                    "name": p.path.split('/').next_back().unwrap_or(&p.path),
                    "type": p.project_type,
                    "tags": tags
                })
            })
            .collect();
        let output = serde_json::json!({
            "query": query,
            "count": results.len(),
            "results": json_results
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "  {}",
            color::info(&format!("Search results for \"{}\"", query))
        );
        println!("  ========================================");
        println!();

        if results.is_empty() {
            println!("  No matching projects.");
            return Ok(());
        }

        for (proj, tags) in &results {
            let name = proj.path.split('/').next_back().unwrap_or(&proj.path);
            let type_colored = match proj.project_type.as_str() {
                "Rust" => color::green(&proj.project_type),
                "JavaScript/TypeScript" => color::yellow(&proj.project_type),
                "Go" => color::blue(&proj.project_type),
                "Python" => color::cyan(&proj.project_type),
                _ => color::white(&proj.project_type),
            };
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!("    tag: {}", color::yellow(&tags.join(", ")))
            };
            println!(
                "    {:<20} {:<20} {}",
                color::cyan(name),
                type_colored,
                tag_str,
            );
        }

        println!();
        println!("  {} results", results.len());
    }

    Ok(())
}
