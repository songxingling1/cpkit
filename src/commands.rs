use crate::{
    cli::{CompileCommand, InitCommand, RunCommand},
    compile::ArtifactBuilder,
    config::get_problem_config_dir,
    configuration::{Config, ProblemConfig},
    print_progress,
    rusage::RusageExt,
    utils::get_signal_name,
};
use anyhow::{Context, Result, anyhow, ensure};
use sha2::Digest;
use std::{os::unix::process::ExitStatusExt, path::Path};
use std::{path::PathBuf, time::Duration};
use tokio::io::AsyncWriteExt;

/// 承接编译请求。
pub async fn compile(
    config: &Config,
    CompileCommand { source, output }: CompileCommand,
) -> Result<()> {
    print_progress!("Compiling", "{}", source.display());
    let mut tmp = ArtifactBuilder::new(&source, Path::new(&config.compile.compiler));
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

/// 峰值内存的除数，在 macOS 上 `ru_maxrss` 单位是 byte，其他平台上是 kb
#[cfg(not(target_os = "macos"))]
const MAXRSS_DIVISOR: f64 = 1024.0;
#[cfg(target_os = "macos")]
const MAXRSS_DIVISOR: f64 = 1024.0 * 1024.0;

/// 承接运行请求
pub async fn run(config: &Config, RunCommand { source }: RunCommand) -> Result<()> {
    ensure!(
        source.is_file(),
        "The source file path should be a valid path."
    );
    let content = tokio::fs::read(&source)
        .await
        .context("Failed to calc output path")?;
    let output = std::env::temp_dir().join(hex::encode(sha2::Sha256::digest(content)));
    if !output.is_file() {
        print_progress!("Compiling", "{}", source.display());
        ArtifactBuilder::new(&source, &config.compile.compiler)
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
    let time = Duration::from_secs(res.rusage.ru_utime.tv_sec.cast_unsigned())
        + Duration::from_secs(res.rusage.ru_stime.tv_sec.cast_unsigned())
        + Duration::from_micros(res.rusage.ru_utime.tv_usec.cast_unsigned())
        + Duration::from_micros(res.rusage.ru_stime.tv_usec.cast_unsigned());
    let time = Duration::from_millis(
        u64::try_from(time.as_millis()).expect("Program execution time exceeded."),
    ); // 截断过于精确的部分
    eprintln!();
    // 显然，一个普通选手程序的运行峰值内存非常难以超过 4096TB（macOS 平台）4096PB（Linux 平台），
    // 所以可以进行安全的强制类型转换。
    #[allow(clippy::cast_precision_loss)]
    let maxrss = res.rusage.ru_maxrss as f64 / MAXRSS_DIVISOR;
    print_progress!(
        "Finished",
        "Execution time: {}  Peak memory: {} MiB  Exit signal: {}.",
        humantime::format_duration(time).to_string(),
        maxrss,
        get_signal_name(res.exit_status.signal())
    );
    Ok(())
}

pub async fn init(
    workspace: Option<&Path>,
    InitCommand {
        source,
        with_checker,
        with_interactive,
    }: InitCommand,
) -> Result<()> {
    let workspace = workspace.ok_or(
        anyhow!("There isn't a workspace folder.")
            .context("Failed to create problem configuration"),
    )?;
    let source = match tokio::fs::canonicalize(source).await {
        Ok(p) => p,
        Err(e) => return Err(anyhow!(e).context("The source file is invalid")),
    };
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
