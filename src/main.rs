use anyhow::Result;
use clap::Parser;
use projector::command::{Commands, ConfigAction, ExportAction, Projector, SnapshotAction};
use projector::subcmd;

fn main() -> Result<()> {
    let cli = Projector::parse();
    match cli.command {
        Commands::List { dir } => {
            subcmd::list::subcmd_list(dir)?;
            Ok(())
        }
        Commands::Scan { dir } => {
            subcmd::scan::subcmd_scan(dir)?;
            Ok(())
        }
        Commands::Report {
            diff,
            format,
            sort,
            filter,
        } => {
            subcmd::report::subcmd_report(diff, format, sort, filter)?;
            Ok(())
        }
        Commands::Config {
            action: Some(ConfigAction::Set { key, value }),
        } => subcmd::config::subcmd_config_set(key, value),
        Commands::Config { action: None } => subcmd::config::subcmd_config_show(),
        Commands::Inspect { path, format } => {
            subcmd::inspect::subcmd_inspect(path, format)?;
            Ok(())
        }
        Commands::Stats { format } => {
            subcmd::stats::subcmd_stats(format)?;
            Ok(())
        }
        Commands::Trend {
            path,
            days,
            metric,
            format,
        } => {
            subcmd::trend::subcmd_trend(path, days, metric, format)?;
            Ok(())
        }
        Commands::Completion { shell } => {
            subcmd::completion::subcmd_completion(shell)?;
            Ok(())
        }
        Commands::Export {
            action: ExportAction::Html { output },
        } => {
            subcmd::export::subcmd_export_html(output)?;
            Ok(())
        }
        Commands::Snapshot {
            action: SnapshotAction::Prune { keep, dry_run },
        } => {
            subcmd::snapshot::subcmd_snapshot_prune(keep, dry_run)?;
            Ok(())
        }
    }
}
