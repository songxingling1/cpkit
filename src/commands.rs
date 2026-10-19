use crate::{
    cli::{CompileCommand, InitCommand, RunCommand},
    compile::ArtifactBuilder,
    configuration::ProblemConfig,
    print_progress,
    runner::{ExitInformation, RunnerExt},
};
use anyhow::{Context, Result, anyhow, ensure};
use std::path::{Path, PathBuf};

/// 承接编译请求。
pub async fn compile(
    ctx: &crate::context::Context,
    CompileCommand { source, output }: CompileCommand,
) -> Result<()> {
    ensure!(source.is_file(), "The source file is not found.");
    let pctx = {
        let mut pctx = ctx
            .get_problem_context(&source)
            .context("Failed to load problem context.")?;
        pctx.load_config()
            .await
            .context("Failed to load problem configuration.")?;
        pctx
    };
    print_progress!("Compiling", "{}", source.display());
    // 已经 Apply 了 problem 并且将错误情况上报了，所以这里一定是 Some
    let config = ctx.get_config().unwrap();
    let mut tmp = ArtifactBuilder::new(
        pctx.get_source_list().unwrap(),
        Path::new(&config.compile.compiler),
    );
    if let Some(output) = output {
        #[cfg(target_os = "windows")]
        let output = output.with_extension("exe");
        tmp = tmp.artifact(&output);
    }
    let res = tmp
        .args(&config.compile.flag)
        .compile()
        .await
        .context("Compilation failed.")?;
    print_progress!(
        "Finished",
        "Successfully compiled {} to {}.",
        source.display(),
        res.display()
    );
    Ok(())
}

/// 承接运行请求
pub async fn run(ctx: &crate::context::Context, RunCommand { source }: RunCommand) -> Result<()> {
    ensure!(
        source.is_file(),
        "The source file path should be a valid path."
    );
    let pctx = {
        let mut pctx = ctx
            .get_problem_context(&source)
            .context("Failed to load problem context.")?;
        pctx.load_config()
            .await
            .context("Failed to load problem configuration.")?;
        pctx
    };
    // 已经 Apply 了 problem 并且将错误情况上报了，所以这里一定是 Some
    let config = ctx.get_config().unwrap();
    let output = ctx
        .get_cache_dir()
        .join(pctx.get_hash().await.context("Compilation failed.")?);
    #[cfg(target_os = "windows")]
    let output = output.with_extension("exe");
    if !output.is_file() {
        print_progress!("Compiling", "{}", source.display());
        ArtifactBuilder::new(pctx.get_source_list().unwrap(), &config.compile.compiler)
            .artifact(&output)
            .args(&config.compile.flag)
            .compile()
            .await
            .context("Compilation failed.")?;
    }
    print_progress!("Running", "{} ({})", output.display(), source.display());
    let mut cmd = tokio::process::Command::new(output).spawn_with_runner()?;
    let ExitInformation {
        time_usage,
        memory_usage,
        status,
    } = tokio::select! {
        res = cmd.wait() => {
            res?
        },
        sig = tokio::signal::ctrl_c() => {
            match sig {
                Ok(()) => {
                    cmd.kill()?;
                    cmd.wait().await?
                },
                Err(e) => {
                    return Err(anyhow!(e).context("Failed to install CTRL+C signal handler"));
                }
            }
        }
    };
    print_progress!(
        "Finished",
        "Execution time: {}  Peak memory: {} MiB  Exit signal: {}.",
        humantime::format_duration(*time_usage).to_string(),
        memory_usage,
        status
    );
    Ok(())
}

pub async fn init(
    ctx: &crate::context::Context,
    InitCommand {
        source,
        with_checker,
        with_interactive,
    }: InitCommand,
) -> Result<()> {
    ensure!(source.is_file(), "The source file is not found.");
    let pctx = ctx
        .get_problem_context(&source)
        .context("Failed to load problem context.")?;
    let mut config = ProblemConfig::default();
    if with_checker {
        config.checker = crate::configuration::Checker::Special {
            path: PathBuf::new(),
        }
    }
    if with_interactive {
        config.problem_type = crate::configuration::ProblemType::Interactive {
            header: PathBuf::new(),
            grader: PathBuf::new(),
        }
    }
    pctx.write_problem_config(&config)
        .await
        .context("Failed to write problem configuration")?;
    print_progress!(
        "Finished",
        "Created configuration for {}.",
        source.display()
    );
    Ok(())
}
