use crate::{
    cli::{CompileCommand, RunCommand},
    compile::ArtifactBuilder,
    configuration::Config,
    print_progress,
    runner::run_with_time_limit,
};
use anyhow::{Context, Result, anyhow};
use std::{os::unix::process::ExitStatusExt, path::Path, time::SystemTime};

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

/// 获取文件修改时间
async fn get_modify_time(path: &Path) -> std::io::Result<SystemTime> {
    tokio::fs::metadata(path).await?.modified()
}

/// 从数字信号获取信号名称
fn get_signal_name(signal: Option<i32>) -> &'static str {
    if let Some(sig) = signal {
        match sig {
            libc::SIGHUP => "Hangup",
            libc::SIGINT => "Interrupt",
            libc::SIGQUIT => "Quit",
            libc::SIGILL => "Illegal instruction",
            libc::SIGTRAP => "Trace/breakpoint trap",
            libc::SIGABRT => "Aborted",
            libc::SIGFPE => "Floating point exception",
            libc::SIGKILL => "Killed",
            libc::SIGBUS => "Bus error",
            libc::SIGSYS => "Bad system call",
            libc::SIGSEGV => "Segmentation fault",
            libc::SIGPIPE => "Broken pipe",
            libc::SIGALRM => "Alarm clock",
            libc::SIGTERM => "Terminated",
            libc::SIGURG => "Urgent I/O condition",
            libc::SIGSTOP => "Stopped (signal)",
            libc::SIGTSTP => "Stopped",
            libc::SIGCONT => "Continued",
            libc::SIGCHLD => "Child exited",
            libc::SIGTTIN => "Stopped (tty input)",
            libc::SIGTTOU => "Stopped (tty output)",
            libc::SIGPOLL => "I/O possible",
            libc::SIGXCPU => "CPU time limit exceeded",
            libc::SIGXFSZ => "File size limit exceeded",
            libc::SIGVTALRM => "Virtual timer expired",
            libc::SIGPROF => "Profiling timer expired",
            libc::SIGUSR1 => "User defined signal 1",
            libc::SIGUSR2 => "User defined signal 2",
            libc::SIGWINCH => "Window changed",
            _ => "Unknown signal",
        }
    } else {
        "None"
    }
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
