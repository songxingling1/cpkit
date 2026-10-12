use libc::{c_int, pid_t, rusage};
use std::{
    borrow::Cow, io::Result, mem::MaybeUninit, os::unix::process::ExitStatusExt,
    process::ExitStatus,
};

#[cfg(not(target_os = "macos"))]
pub const MAXRSS_DIVISOR: f64 = 1024.0;
#[cfg(target_os = "macos")]
pub const MAXRSS_DIVISOR: f64 = 1024.0 * 1024.0;

fn cvt(pid: pid_t) -> Result<pid_t> {
    if pid == -1 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(pid)
    }
}

pub fn wait4(pid: pid_t, options: c_int) -> Result<Option<(ExitStatus, rusage)>> {
    let mut status: c_int = 0;
    let mut rusage = MaybeUninit::<rusage>::uninit();
    // SAFETY: status 和 rusage 均为有效的指针，所以 wait4 是安全的。
    let res = cvt(unsafe { libc::wait4(pid, &raw mut status, options, rusage.as_mut_ptr()) })?;
    if res == 0 {
        Ok(None)
    } else {
        Ok(Some((ExitStatus::from_raw(status), unsafe {
            // SAFETY: status 和 rusage 都不是空指针并且 wait4 已经正确返回，所以此时初始化是安全的
            rusage.assume_init()
        })))
    }
}

pub fn strsignal(sig: Option<i32>) -> Cow<'static, str> {
    if let Some(sig) = sig {
        match sig {
            libc::SIGHUP => Cow::Borrowed("Hangup"),
            libc::SIGINT => Cow::Borrowed("Interrupt"),
            libc::SIGQUIT => Cow::Borrowed("Quit"),
            libc::SIGILL => Cow::Borrowed("Illegal instruction"),
            libc::SIGTRAP => Cow::Borrowed("Trace/breakpoint trap"),
            libc::SIGABRT => Cow::Borrowed("Aborted"),
            libc::SIGFPE => Cow::Borrowed("Floating point exception"),
            libc::SIGKILL => Cow::Borrowed("Killed"),
            libc::SIGBUS => Cow::Borrowed("Bus error"),
            libc::SIGSYS => Cow::Borrowed("Bad system call"),
            libc::SIGSEGV => Cow::Borrowed("Segmentation fault"),
            libc::SIGPIPE => Cow::Borrowed("Broken pipe"),
            libc::SIGALRM => Cow::Borrowed("Alarm clock"),
            libc::SIGTERM => Cow::Borrowed("Terminated"),
            libc::SIGURG => Cow::Borrowed("Urgent I/O condition"),
            libc::SIGSTOP => Cow::Borrowed("Stopped (signal)"),
            libc::SIGTSTP => Cow::Borrowed("Stopped"),
            libc::SIGCONT => Cow::Borrowed("Continued"),
            libc::SIGCHLD => Cow::Borrowed("Child exited"),
            libc::SIGTTIN => Cow::Borrowed("Stopped (tty input)"),
            libc::SIGTTOU => Cow::Borrowed("Stopped (tty output)"),
            libc::SIGPOLL => Cow::Borrowed("I/O possible"),
            libc::SIGXCPU => Cow::Borrowed("CPU time limit exceeded"),
            libc::SIGXFSZ => Cow::Borrowed("File size limit exceeded"),
            libc::SIGVTALRM => Cow::Borrowed("Virtual timer expired"),
            libc::SIGPROF => Cow::Borrowed("Profiling timer expired"),
            libc::SIGUSR1 => Cow::Borrowed("User defined signal 1"),
            libc::SIGUSR2 => Cow::Borrowed("User defined signal 2"),
            libc::SIGWINCH => Cow::Borrowed("Window changed"),
            _ => Cow::Owned(format!("Unknown signal ({sig})")),
        }
    } else {
        Cow::Borrowed("None")
    }
}
