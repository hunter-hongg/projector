use anyhow::Result;

use crate::color;
use crate::tags::TagsIndex;

pub fn subcmd_tag_list(path: Option<String>) -> Result<()> {
    let index = TagsIndex::load()?;

    match path {
        Some(p) => {
            let tags = index.tags_for_path(&p);
            if tags.is_empty() {
                println!("  {} has no tags", color::cyan(&p));
            } else {
                println!("  Tags for {}:", color::cyan(&p));
                for t in &tags {
                    println!("    {}", color::green(t));
                }
            }
        }
        None => {
            let all_tags = index.all_tags();
            if all_tags.is_empty() {
                println!("  No tags defined. Use `projector tag set <path> <tag>` to add one.");
            } else {
                println!("  All tags:");
                for t in &all_tags {
                    let count = index.paths_for_tag(t).len();
                    println!("    {} ({})", color::green(t), count);
                }
            }
        }
    }
    Ok(())
}

pub fn subcmd_tag_set(path: String, tag: String) -> Result<()> {
    if tag.trim().is_empty() || tag.contains(' ') {
        anyhow::bail!("Tag name must not be empty or contain spaces");
    }

    let mut index = TagsIndex::load()?;
    if !index.has_tag(&path, &tag) {
        index.add_tag(&path, &tag);
        index.save()?;
    }
    println!("  Tagged {} with {}", color::cyan(&path), color::green(&tag));
    Ok(())
}

pub fn subcmd_tag_rm(path: String, tag: String) -> Result<()> {
    let mut index = TagsIndex::load()?;
    index.remove_tag(&path, &tag);
    index.save()?;
    println!(
        "  Removed tag {} from {}",
        color::green(&tag),
        color::cyan(&path)
    );
    Ok(())
}

pub fn subcmd_tag_clear(path: String) -> Result<()> {
    let mut index = TagsIndex::load()?;
    index.clear_path(&path);
    index.save()?;
    println!("  Cleared all tags from {}", color::cyan(&path));
    Ok(())
}
