use clap::Parser;
use projector::command::{ConfigAction, Projector, Commands};
use projector::global;
use projector::subcmd;
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Projector::parse();
    match cli.command {
        Commands::Version => {
            println!("Projector version {}", global::version());
            Ok(())
        }
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
        Commands::Config { action } => {
            match action {
                Some(ConfigAction::Set { key, value }) => {
                    subcmd::config::subcmd_config_set(key, value)?;
                }
                None => {
                    subcmd::config::subcmd_config_show()?;
                }
            }
            Ok(())
        }
    }
}
