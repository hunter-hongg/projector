use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "projector")]
#[command(about = "📊 统计个人项目并提供分析", long_about = None)]
pub struct Projector {
  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  Version,
}