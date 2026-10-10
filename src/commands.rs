use crate::{
    cli::{CompileCommand, InitCommand, RunCommand},
    compile::ArtifactBuilder,
    configuration::ProblemConfig,
    print_progress,
    rusage::RusageExt,
    utils::calc_run_result,
};
use anyhow::{Context, Result, anyhow, ensure};
use std::path::{Path, PathBuf};

/// 承接编译请求。
pub async fn compile(
    ctx: &mut crate::config::Context,
    CompileCommand { source, output }: CompileCommand,
) -> Result<()> {
    ensure!(source.is_file(), "The source file is not found.");
    let token = ctx
        .get_problem_token(&source)
        .context("Failed to find problem configuration.")?;
    ctx.load_problem_config(&token)
        .await
        .context("Failed to apply problem configuration.")?;
    print_progress!("Compiling", "{}", source.display());
    // 已经 Apply 了 problem 并且将错误情况上报了，所以这里一定是 Some
    let config = ctx.get_global_config().unwrap();
    let mut tmp = ArtifactBuilder::new(
        ctx.get_source_list().unwrap(),
        Path::new(&config.compile.compiler),
    );
    if let Some(output) = output {
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
pub async fn run(
    ctx: &mut crate::config::Context,
    RunCommand { source }: RunCommand,
) -> Result<()> {
    ensure!(
        source.is_file(),
        "The source file path should be a valid path."
    );
    let token = ctx
        .get_problem_token(&source)
        .context("Failed to find problem configuration.")?;
    ctx.load_problem_config(&token)
        .await
        .context("Failed to apply problem configuration.")?;
    // 已经 Apply 了 problem 并且将错误情况上报了，所以这里一定是 Some
    let config = ctx.get_global_config().unwrap();
    let output = ctx.get_output_path().await.context("Compilation failed.")?;
    if !output.is_file() {
        print_progress!("Compiling", "{}", source.display());
        ArtifactBuilder::new(ctx.get_source_list().unwrap(), &config.compile.compiler)
            .artifact(&output)
            .args(&config.compile.flag)
            .compile()
            .await
            .context("Compilation failed.")?;
    }
    print_progress!("Running", "{} ({})", output.display(), source.display());
    let mut cmd = std::process::Command::new(output).spawn_with_rusage()?;
    let res = tokio::select! {
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
    let (time, maxrss, signal) = calc_run_result(&res);
    print_progress!(
        "Finished",
        "Execution time: {}  Peak memory: {} MiB  Exit signal: {}.",
        humantime::format_duration(time).to_string(),
        maxrss,
        signal
    );
    Ok(())
}

pub async fn init(
    ctx: &mut crate::config::Context,
    InitCommand {
        source,
        with_checker,
        with_interactive,
    }: InitCommand,
) -> Result<()> {
    ensure!(source.is_file(), "The source file is not found.");
    let token = ctx
        .get_problem_token(&source)
        .context("Failed to calc configuration location.")?;
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
    ctx.write_problem_config(&token, &config)
        .await
        .context("Failed to write problem configuration")?;
    print_progress!(
        "Finished",
        "Created configuration for {}.",
        source.display()
    );
    Ok(())
}
