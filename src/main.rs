use anyhow::Result;
use clap::Parser;
use projector::command::{Commands, ConfigAction, Projector};
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
        Commands::Report { diff, format } => {
            subcmd::report::subcmd_report(diff, format)?;
            Ok(())
        }
        Commands::Config {
            action: Some(ConfigAction::Set { key, value }),
        } => subcmd::config::subcmd_config_set(key, value),
        Commands::Config { action: None } => subcmd::config::subcmd_config_show(),
    }
}
