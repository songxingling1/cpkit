use super::model::ExitInformation;
use libc::{rusage, siginfo_t};
use std::ffi::c_long;
use std::mem::MaybeUninit;
use std::process::ExitStatus;
use std::{borrow::Cow, io::Result};
use std::{os::unix::process::ExitStatusExt, time::Duration};
use tokio::signal::unix::{Signal, SignalKind, signal};

/// 运行器（使用 Pidfd 实现）
pub struct RunnerChild {
    child: tokio::process::Child,
    result: Option<ExitInformation>,
    sig: Signal,
}

impl RunnerChild {
    pub fn new(child: tokio::process::Child) -> Result<RunnerChild> {
        Ok(RunnerChild {
            child,
            result: None,
            sig: signal(SignalKind::child())?,
        })
    }
    /// 击杀进程（发送 Kill 信号）
    pub fn kill(&mut self) -> Result<()> {
        if self.result.is_some() {
            Ok(())
        } else {
            self.child.start_kill()
        }
    }
    /// 尝试等待进程退出
    pub fn try_wait(&mut self) -> Result<Option<&ExitInformation>> {
        if self.result.is_none()
            && let Some((e, r)) = waitid(
                self.child.id().unwrap(), // 显然因为 result 是空的所以 child 没有被 Reap。
                libc::WNOHANG | libc::WNOWAIT | libc::WEXITED,
            )?
        {
            let time_usage = Duration::from_secs(r.ru_utime.tv_sec.cast_unsigned())
                + Duration::from_secs(r.ru_stime.tv_sec.cast_unsigned())
                + Duration::from_micros(r.ru_utime.tv_usec.cast_unsigned())
                + Duration::from_micros(r.ru_stime.tv_usec.cast_unsigned());
            #[allow(clippy::cast_precision_loss)]
            let memory_usage = r.ru_maxrss as f64 / MAXRSS_DIVISOR;
            self.result = Some(ExitInformation {
                time_usage,
                memory_usage,
                status: strsignal(e.signal()),
            });
            self.child
                .try_wait()
                .expect("If the first wait succeeds, the second wait cannot fail.")
                .expect("If the first wait succeeds, the second wait cannot fail.");
        }
        Ok(self.result.as_ref())
    }
    // 等待进程退出，允许被取消
    pub async fn wait(&mut self) -> Result<&ExitInformation> {
        if self.result.is_none() {
            loop {
                if self.try_wait()?.is_some() {
                    break;
                }
                self.sig.recv().await;
            }
        }
        Ok(self.result.as_ref().unwrap())
    }
    #[allow(dead_code)]
    pub fn stdin(&mut self) -> Option<tokio::process::ChildStdin> {
        self.child.stdin.take()
    }
    #[allow(dead_code)]
    pub fn stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.child.stdout.take()
    }
    #[allow(dead_code)]
    pub fn stderr(&mut self) -> Option<tokio::process::ChildStderr> {
        self.child.stderr.take()
    }
}

fn cvt<T: PartialEq + From<i32>>(pid: T) -> Result<T> {
    if pid == T::from(-1) {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(pid)
    }
}

pub const MAXRSS_DIVISOR: f64 = 1024.0;

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

/// waitid 封装。在传入非 WEXITED 类型的筛选参数的时候有可能 panic。
pub fn waitid(pid: u32, options: libc::c_int) -> Result<Option<(ExitStatus, rusage)>> {
    let mut siginfo = MaybeUninit::<siginfo_t>::zeroed();
    let mut ru = MaybeUninit::<rusage>::uninit();
    cvt(unsafe {
        libc::syscall(
            libc::SYS_waitid,
            c_long::from(libc::P_PID),
            c_long::from(pid),
            siginfo.as_mut_ptr(),
            c_long::from(options),
            ru.as_mut_ptr(),
        )
    })?;
    // SAFETY：siginfo 已经预先填充为 0，并且先回判断 signo 是否为 0，如果
    // 是 0，就会立刻返回不会使用其他字段。否则，整个结构体就是合法的。
    let siginfo = unsafe { siginfo.assume_init() };
    // SAFETY：如果没有进程满足条件，那么 siginfo 的 si_pid 字段一定是一个合法值 0
    if unsafe { siginfo.si_pid() } == 0 {
        return Ok(None);
    }
    if siginfo.si_signo != libc::SIGCHLD {
        unreachable!("waitid returned unexpected si_signo: {}", siginfo.si_signo);
    }
    // SAFETY：由于已经判断了有子进程被 waitid 检测到，所以 siginfo 的剩下的所有字段都是合法的。
    let si_status = unsafe { siginfo.si_status() };
    let exit = match siginfo.si_code {
        libc::CLD_EXITED => ExitStatus::from_raw((si_status & 0xff) << 8),
        libc::CLD_KILLED => ExitStatus::from_raw(si_status),
        libc::CLD_DUMPED => ExitStatus::from_raw(si_status | 0x80),
        c => unreachable!("Unexpected siginfo si_code: {}", c),
    };
    // SAFETY：由于判掉了系统调用失败、没有符合条件的进程、
    // 退出状态不是正常退出或者信号杀死的情况，所以 rusage 一定是合法值。
    let ru = unsafe { ru.assume_init() };
    Ok(Some((exit, ru)))
}
