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

/// Rewrite snapshots left behind by an older projector at the current schema.
///
/// Loading old snapshots already works without this (every field has a serde
/// default); migrating just persists the backfill so files stop being patched
/// in memory on every read.
pub fn subcmd_snapshot_migrate() -> Result<()> {
    let report = SnapshotStore::migrate_all()?;

    if report.is_empty() {
        println!("{}", color::info("No snapshots found to migrate."));
        return Ok(());
    }

    if report.migrated.is_empty() {
        println!(
            "{}",
            color::info(&format!(
                "All {} snapshot(s) are already at schema version {}.",
                report.current,
                crate::snapshot::SCHEMA_VERSION
            ))
        );
    } else {
        println!(
            "{}",
            color::info(&format!(
                "Migrated {} snapshot(s) to schema version {} ({} already current).",
                report.migrated.len(),
                crate::snapshot::SCHEMA_VERSION,
                report.current
            ))
        );
        for path in &report.migrated {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            println!("  {}", color::green(name));
        }
    }

    for path in &report.failed {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        println!("  {}", color::red(&format!("unreadable: {name}")));
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
