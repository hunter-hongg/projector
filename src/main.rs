use clap::Parser;
use projector::command::{Projector, Commands};
use projector::global;

fn main() {
    let cli = Projector::parse();
    match cli.command {
        Commands::Version => {
            println!("Projector version {}", global::version());
        }
    }
}
