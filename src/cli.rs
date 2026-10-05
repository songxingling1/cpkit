use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cpkit {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Args)]
pub struct CompileCommand {
    pub source: PathBuf,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// 子命令
#[derive(Subcommand)]
pub enum Commands {
    /// 编译源文件
    Compile(CompileCommand),
}
