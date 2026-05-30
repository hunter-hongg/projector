use clap::{Parser, Subcommand, ValueEnum};

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
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        filter: Vec<String>,
    },
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    Inspect {
        path: Option<String>,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Stats {
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Trend {
        path: Option<String>,
        #[arg(long)]
        days: Option<u32>,
        #[arg(long)]
        metric: Option<String>,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Completion {
        shell: ShellKind,
    },
    Export {
        #[command(subcommand)]
        action: ExportAction,
    },
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Set { key: String, value: String },
}

#[derive(Subcommand)]
pub enum ExportAction {
    Html {
        #[arg(short = 'o', long = "output")]
        output: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
}

#[derive(Subcommand)]
pub enum SnapshotAction {
    Prune {
        #[arg(long)]
        keep: Option<u32>,
        #[arg(long)]
        dry_run: bool,
    },
}
