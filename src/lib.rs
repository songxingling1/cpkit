mod cli;
mod commands;
mod compile;
mod config;
mod configuration;
pub mod print;
use anyhow::Context;
use clap::Parser;
use cli::*;

use crate::{config::LoadConfig, configuration::Config, print::print_warn};

pub fn app() -> anyhow::Result<()> {
    let cpkit = Cpkit::parse();
    let config = Config::from_dir(
        &std::env::current_dir().context("Failed to get current directory.")?,
        "cpkit.toml",
    )
    .unwrap_or_else(|_| {
        print_warn("Failed to load configuration. Use default configuration.");
        Config::default()
    });
    match cpkit.commands {
        Commands::Compile { source, output } => {
            commands::compile(&config, &source, output.as_deref())
        }
    }?;
    Ok(())
}
