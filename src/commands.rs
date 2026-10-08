use crate::{
    cli::{CompileCommand, RunCommand},
    compile::ArtifactBuilder,
    configuration::Config,
    print_progress,
    utils::{get_modify_time, get_signal_name, run_with_time_limit},
};
use anyhow::{Context, Result, anyhow};
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

/// 峰值内存的除数，在 macOS 上 ru_maxrss 单位是 byte，其他平台上是 kb
#[cfg(not(target_os = "macos"))]
const MAXRSS_DIVISOR: f64 = 1024.0;
#[cfg(target_os = "macos")]
const MAXRSS_DIVISOR: f64 = 1024.0 * 1024.0;

/// 承接运行请求
pub async fn run(config: &Config, RunCommand { source, limit }: RunCommand) -> Result<()> {
    let output = source
        .file_prefix()
        .ok_or(anyhow!("The source file path should be a valid path."))?;
    let output = std::env::temp_dir().join(output);
    let source_time = get_modify_time(&source)
        .await
        .context("Failed to get the source file modification time.")?;
    let output_time = get_modify_time(&output)
        .await
        .context("Failed to get the output file modification time.")?;
    if !output.is_file() || source_time > output_time {
        print_progress!("Compiling", "{}", source.display());
        ArtifactBuilder::new(&source, &config.compile.compiler)
            .artifact(&output)
            .args(&config.compile.flag)
            .compile()
            .await
            .context("Compilation failed.")?;
    }
    print_progress!("Running", "{}", output.display());
    let mut cmd = std::process::Command::new(output);
    let res = run_with_time_limit(&mut cmd, limit).await?;
    let time = std::time::Duration::new(
        (res.rusage.ru_utime.tv_sec + res.rusage.ru_stime.tv_sec) as u64,
        ((res.rusage.ru_utime.tv_usec + res.rusage.ru_stime.tv_usec) * 1000) as u32,
    );
    let time = std::time::Duration::from_millis(time.as_millis() as u64); // 截断过于精确的部分
    eprint!("\n");
    print_progress!(
        "Finished",
        "Execution time: {}  Peak memory: {} MiB  Exit signal: {}.",
        humantime::format_duration(time).to_string(),
        res.rusage.ru_maxrss as f64 / MAXRSS_DIVISOR,
        get_signal_name(res.exit_status.signal())
    );
    Ok(())
}
