mod cli;
mod compile;
mod print;
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
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn app() -> anyhow::Result<()> {
    let cpkit = Cpkit::parse();
    match cpkit.commands {
        Commands::Compile { source, output } => cli::compile(&source, output.as_deref()),
    }?;
    Ok(())
}
