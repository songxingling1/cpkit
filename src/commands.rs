use crate::{
    cli::{CompileCommand, RunCommand},
    compile::ArtifactBuilder,
    configuration::Config,
    print_progress,
    rusage::RusageExt,
    utils::{get_modify_time, get_signal_name},
};
use anyhow::{Context, Result, anyhow, ensure};
use std::time::Duration;
use std::{os::unix::process::ExitStatusExt, path::Path};

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
    let output = source
        .file_prefix()
        .ok_or(anyhow!("The source file path should be a valid path."))?;
    let output = std::env::temp_dir().join(output);
    let source_time = get_modify_time(&source)
        .await
        .context("Failed to get the source file modification time.");
    let output_time = get_modify_time(&output)
        .await
        .context("Failed to get the output file modification time.");
    if !output.is_file() || source_time? > output_time? {
        print_progress!("Compiling", "{}", source.display());
        ArtifactBuilder::new(&source, &config.compile.compiler)
            .artifact(&output)
            .args(&config.compile.flag)
            .compile()
            .await
            .context("Compilation failed.")?;
    }
    print_progress!("Running", "{}", output.display());
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
