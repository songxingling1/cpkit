use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cpkit {
    #[command(subcommand)]
    pub commands: Commands,
}

/// 子命令
#[derive(Subcommand)]
pub enum Commands {
    /// 编译源文件
    Compile {
        source: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
