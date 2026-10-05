use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// CLI
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cpkit {
    #[command(subcommand)]
    pub commands: Commands,
}

/// 编译命令
#[derive(Args)]
pub struct CompileCommand {
    /// 源码
    pub source: PathBuf,
    /// 产物位置
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// 子命令
#[derive(Subcommand)]
pub enum Commands {
    /// 编译源文件
    Compile(CompileCommand),
}
