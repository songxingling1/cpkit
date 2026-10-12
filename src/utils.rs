use std::{os::unix::process::ExitStatusExt, time::Duration};

use crate::rusage::ExitInformation;

#[must_use]
pub fn vecu8_to_string_lossy(vec: Vec<u8>) -> String {
    match String::from_utf8(vec) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into_owned(),
    }
}

/// 峰值内存的除数，在 macOS 上 `ru_maxrss` 单位是 byte，其他平台上是 kb
#[cfg(not(target_os = "macos"))]
const MAXRSS_DIVISOR: f64 = 1024.0;
#[cfg(target_os = "macos")]
const MAXRSS_DIVISOR: f64 = 1024.0 * 1024.0;

pub fn calc_run_result(res: &ExitInformation) -> (Duration, f64, &'static str) {
    let time = Duration::from_secs(res.rusage.ru_utime.tv_sec.cast_unsigned())
        + Duration::from_secs(res.rusage.ru_stime.tv_sec.cast_unsigned())
        + Duration::from_micros(res.rusage.ru_utime.tv_usec.cast_unsigned())
        + Duration::from_micros(res.rusage.ru_stime.tv_usec.cast_unsigned());
    let time = Duration::from_millis(
        u64::try_from(time.as_millis()).expect("Program execution time exceeded."),
    ); // 截断过于精确的部分
    // 显然，一个普通选手程序的运行峰值内存非常难以超过 4096TB（macOS 平台）4096PB（Linux 平台），
    // 所以可以进行安全的强制类型转换。
    #[allow(clippy::cast_precision_loss)]
    let maxrss = res.rusage.ru_maxrss as f64 / MAXRSS_DIVISOR;
    let signal: &'static str = if let Some(sig) = res.exit_status.signal() {
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
    };
    (time, maxrss, signal)
}

#[cfg(test)]
#[path = "tests/test_utils.rs"]
mod tests;
