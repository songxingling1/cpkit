use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Cpkit
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cpkit {
    #[command(subcommand)]
    pub commands: Commands,
}

/// Compile Command
#[derive(Args)]
pub struct CompileCommand {
    /// Source
    pub source: PathBuf,
    /// Output Path
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args)]
pub struct RunCommand {
    /// Source
    pub source: PathBuf,
    #[arg(short, long,value_parser = humantime::parse_duration,
        default_value = "1s")]
    pub limit: std::time::Duration,
}

/// 子命令
#[derive(Subcommand)]
pub enum Commands {
    /// Compile the source file.
    Compile(CompileCommand),
    /// Run the source file.
    Run(RunCommand),
}
