mod cli;
mod commands;
mod compile;
mod print;
use clap::Parser;
use cli::*;

pub fn app() -> anyhow::Result<()> {
    let cpkit = Cpkit::parse();
    match cpkit.commands {
        Commands::Compile { source, output } => commands::compile(&source, output.as_deref()),
    }?;
    Ok(())
}
