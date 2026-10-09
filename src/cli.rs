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

/// Run Command
#[derive(Args)]
pub struct RunCommand {
    /// Source
    pub source: PathBuf,
}

#[derive(Args)]
pub struct InitCommand {
    /// Source
    pub source: PathBuf,
    /// Interactive
    #[arg(long, short = 'i')]
    pub with_interactive: bool,
    /// Special judge
    #[arg(long, short = 'c')]
    pub with_checker: bool,
}

/// 子命令
#[derive(Subcommand)]
pub enum Commands {
    /// Compile the source file.
    Compile(CompileCommand),
    /// Run the source file.
    Run(RunCommand),
    /// Init config for problem
    Init(InitCommand),
}
