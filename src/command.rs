use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "projector", version, about = "Analyze and manage your personal projects", long_about = None)]
pub struct Projector {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    List {
        dir: Option<String>,
        #[arg(long)]
        tag: Option<String>,
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
    Activity {
        #[arg(long)]
        days: Option<u32>,
        #[arg(long)]
        project: Option<String>,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Brief {
        #[arg(long, default_value = "1")]
        days: u32,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Deps {
        path: Option<String>,
        #[arg(long)]
        shared: bool,
        #[arg(long)]
        project: Option<String>,
        /// Ask each ecosystem's own toolchain whether newer versions exist.
        /// Reaches the network; off unless this flag is passed.
        #[arg(long)]
        outdated: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Orphans {
        #[arg(long)]
        days: Option<u32>,
        #[arg(long)]
        all: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Rank {
        #[arg(long)]
        by: Option<String>,
        #[arg(long)]
        reverse: bool,
        #[arg(long = "type")]
        type_filter: Option<String>,
        #[arg(long)]
        top: Option<usize>,
        #[arg(long)]
        category: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Search {
        query: String,
        #[arg(long)]
        tag: Option<String>,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
    },
    Size {
        path: Option<String>,
        #[arg(long)]
        top: Option<usize>,
        #[arg(long)]
        deep: bool,
        #[arg(short = 'f', long = "format")]
        format: Option<String>,
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
    Tag {
        #[command(subcommand)]
        action: TagAction,
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
    /// Render the dashboard as a Markdown document.
    Markdown {
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
    /// Rewrite snapshots written by an older projector at the current schema
    Migrate,
}

#[derive(Subcommand)]
pub enum TagAction {
    List { path: Option<String> },
    Set { path: String, tag: String },
    Rm { path: String, tag: String },
    Clear { path: String },
}
