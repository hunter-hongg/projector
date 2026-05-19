use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "projector", version, about = "统计个人项目并提供分析", long_about = None)]
pub struct Projector {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List {
        dir: Option<String>,
    },
    Scan {
        dir: Option<String>,
    },
    Report {
        #[arg(long)]
        diff: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Set { key: String, value: String },
}
