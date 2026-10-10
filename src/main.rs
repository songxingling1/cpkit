mod cli;
mod commands;
mod compile;
mod config;
mod configuration;
mod print;
mod rusage;
mod utils;
use anyhow::Context;
use clap::Parser;
use cli::{Commands, Cpkit};

/// CLI 入口
/// # Errors
/// 当 CLI 运行过程中产生了任何错误，返回 `Error`，除非主动 `panic!`。
pub async fn app() -> anyhow::Result<()> {
    let cpkit = Cpkit::parse();
    let mut ctx = config::Context::new().context("Failed to load context.")?;
    ctx.load_global_config()
        .await
        .context("Failed to load global config")?;
    match cpkit.commands {
        Commands::Compile(arg) => commands::compile(&mut ctx, arg).await,
        Commands::Run(arg) => commands::run(&mut ctx, arg).await,
        Commands::Init(arg) => commands::init(&mut ctx, arg).await,
    }?;
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = app().await {
        print_error!("{e:?}");
        std::process::exit(1);
    }
}
