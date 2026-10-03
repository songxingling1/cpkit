mod compile;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cpkit {
    #[command(subcommand)]
    commands: Commands,
}

/// 子命令
#[derive(Subcommand)]
enum Commands {
    /// 编译源文件
    Compile {
        source: PathBuf,
        #[arg(short, long, required = true)]
        output: PathBuf,
    },
}

pub fn app() -> anyhow::Result<()> {
    let cpkit = Cpkit::parse();
    Ok(())
}
