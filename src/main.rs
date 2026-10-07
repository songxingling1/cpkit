mod cli;
mod commands;
mod compile;
mod config;
mod configuration;
pub mod print;
use anyhow::Context;
use clap::Parser;
use cli::{Commands, Cpkit};

use crate::configuration::Config;

/// CLI 入口
/// # Errors
/// 当 CLI 运行过程中产生了任何错误，返回 `Error`，除非主动 `panic!`。
pub async fn app() -> anyhow::Result<()> {
    let cpkit = Cpkit::parse();
    let workspace = config::discover_workspace(
        &std::env::current_dir().context("Failed to get current directory.")?,
        "cpkit.toml",
    );
    let config = if let Some(w) = workspace {
        config::load_config(&w.join("cpkit.toml"))
            .await
            .unwrap_or_else(|_| {
                print_warn!("Failed to load configuration. Use default configuration.");
                Config::default()
            })
    } else {
        print_warn!("Failed to find workspace folder. Use default configuration.");
        Config::default()
    };
    match cpkit.commands {
        Commands::Compile(arg) => commands::compile(&config, arg),
    }
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = app().await {
        print_error!("{e:?}");
        std::process::exit(1);
    }
}
