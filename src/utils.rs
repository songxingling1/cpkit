#[must_use]
pub fn vecu8_to_string_lossy(vec: Vec<u8>) -> String {
    match String::from_utf8(vec) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into_owned(),
    }
}

/// 从数字信号获取信号名称
pub fn get_signal_name(signal: Option<i32>) -> &'static str {
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

#[cfg(test)]
#[path = "tests/test_utils.rs"]
mod tests;
