use crate::{
    cli::{CompileCommand, InitCommand, RunCommand},
    compile::ArtifactBuilder,
    config::{get_problem_config_dir, load_config},
    configuration::{
        Config, ProblemConfig,
        ProblemType::{Interactive, Tranditional},
    },
    print_progress,
    rusage::RusageExt,
    utils::{calc_file_hash, calc_run_result, to_absolute_path_checked},
};
use anyhow::{Context, Result, anyhow, ensure};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// 承接编译请求。
pub async fn compile(
    config: &Config,
    workspace: &Path,
    CompileCommand { source, output }: CompileCommand,
) -> Result<()> {
    let source = to_absolute_path_checked(&source)
        .await
        .context("The source file should exist and be a file.")?;
    let problem_config_dir = get_problem_config_dir(&source, workspace, ".cpkit")
        .context("Failed to read problem configuration.")?;
    let problem: ProblemConfig = load_config(&problem_config_dir.join("config.toml")).await?;
    print_progress!("Compiling", "{}", source.display());
    let mut tmp = match problem.problem_type {
        Tranditional { name: _ } => {
            ArtifactBuilder::new([&source], Path::new(&config.compile.compiler))
        }
        Interactive { header: _, grader } => ArtifactBuilder::new(
            [&source, &problem_config_dir.join(grader)],
            Path::new(&config.compile.compiler),
        ),
    };
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
pub async fn run(config: &Config, RunCommand { source }: RunCommand) -> Result<()> {
    ensure!(
        source.is_file(),
        "The source file path should be a valid path."
    );
    let output = std::env::temp_dir().join(
        calc_file_hash(&source)
            .await
            .context("Failed to calc temporary file name")?,
    );
    if !output.is_file() {
        print_progress!("Compiling", "{}", source.display());
        ArtifactBuilder::new([&source], &config.compile.compiler)
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
    workspace: &Path,
    InitCommand {
        source,
        with_checker,
        with_interactive,
    }: InitCommand,
) -> Result<()> {
    let source = to_absolute_path_checked(&source)
        .await
        .context("The source file should exist and be a file.")?;
    let dir = get_problem_config_dir(&source, workspace, ".cpkit")
        .context("Failed to compute the problem configuration path.")?;
    tokio::fs::create_dir_all(&dir)
        .await
        .context("Failed to create problem configuration")?;
    let mut file = tokio::fs::File::create_new(dir.join("config.toml"))
        .await
        .context("Failed to create problem configuration")?;
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
    file.write_all(
        toml::to_string_pretty(&config)
            .context("Failed to generate configuration template.")?
            .as_bytes(),
    )
    .await
    .context("Failed to write configuration.")?;
    print_progress!(
        "Finished",
        "Created configuration for {}.",
        source.display()
    );
    Ok(())
}
