use clap::Parser;
use projector::command::{Projector, Commands};
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
    }
}
