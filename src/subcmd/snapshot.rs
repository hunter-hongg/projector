use anyhow::Result;

use crate::color;
use crate::config::Config;
use crate::snapshot::SnapshotStore;

pub fn subcmd_snapshot_prune(keep: Option<u32>, dry_run: bool) -> Result<()> {
    let config = Config::load()?;
    let keep = keep.unwrap_or(config.snapshot.keep_count);

    if keep < 1 {
        anyhow::bail!("--keep must be at least 1");
    }

    let removed = SnapshotStore::prune(keep, dry_run)?;

    if removed.is_empty() {
        println!(
            "{}",
            color::info(&format!(
                "No snapshots to prune ({} total, keeping {})",
                SnapshotStore::load_all()?.len(),
                keep
            ))
        );
        return Ok(());
    }

    if dry_run {
        println!(
            "{}",
            color::info(&format!(
                "Would remove {} snapshot(s) (keeping {})",
                removed.len(),
                keep
            ))
        );
    } else {
        println!(
            "{}",
            color::info(&format!(
                "Removed {} snapshot(s) (keeping {})",
                removed.len(),
                keep
            ))
        );
    }

    for path in &removed {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        if dry_run {
            println!("  would remove: {}", color::yellow(name));
        } else {
            println!("  removed: {}", color::red(name));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_subcmd_snapshot_prune_keep_zero_errors() {
        let result = super::subcmd_snapshot_prune(Some(0), true);
        if let Err(e) = &result {
            eprintln!("Error: {}", e);
        }
        assert!(result.is_err());
    }
}
