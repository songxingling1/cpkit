use super::model::ExitInformation;
use block2::RcBlock;
use dispatch2::{
    _dispatch_source_type_proc, DispatchObject, DispatchQoS, DispatchQueue, DispatchRetained,
    DispatchSource, dispatch_source_proc_flags_t,
};
use libproc::pid_rusage::pidrusage;
use std::borrow::Cow;
use std::io::Result;
use std::sync::Mutex;
use std::{os::unix::process::ExitStatusExt, time::Duration};
use tokio::sync::oneshot;

/// 运行器（使用 Pidfd 实现）
pub struct RunnerChild {
    child: tokio::process::Child,
    result: Option<ExitInformation>,
    source: DispatchRetained<DispatchSource>,
    rx: oneshot::Receiver<()>,
}

impl RunnerChild {
    pub fn new(child: tokio::process::Child) -> Result<Self> {
        let pid = child.id().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "The child process shouldn't be reaped.",
        ))?;
        let source = unsafe {
            DispatchSource::new(
                (&raw const _dispatch_source_type_proc).cast_mut(),
                pid as _,
                dispatch_source_proc_flags_t::DISPATCH_PROC_EXIT
                    .0
                    .try_into()
                    .unwrap(),
                Some(&DispatchQueue::global_queue(
                    dispatch2::GlobalQueueIdentifier::QualityOfService(DispatchQoS::Default),
                )),
            )
        };
        let (tx, rx) = oneshot::channel();
        let sender = Mutex::new(Some(tx));
        let cb = RcBlock::new(move || {
            if let Some(tx) = sender.lock().unwrap().take() {
                let _ = tx.send(());
            }
        });
        unsafe {
            source.set_event_handler_with_block(RcBlock::as_ptr(&cb));
        }
        source.activate();
        Ok(Self {
            child,
            result: None,
            source,
            rx,
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
        if self.result.is_none() && self.rx.try_recv().is_ok() {
            self.get_result()?;
        }
        Ok(self.result.as_ref())
    }
    // 等待进程退出，允许被取消
    pub async fn wait(&mut self) -> Result<&ExitInformation> {
        if self.result.is_none() {
            let _ = (&mut self.rx).await;
            self.get_result()?;
        }
        Ok(self.result.as_ref().unwrap())
    }
    fn get_result(&mut self) -> Result<()> {
        let r =
            pidrusage::<libproc::pid_rusage::RUsageInfoV4>(self.child.id().unwrap().cast_signed())
                .map_err(std::io::Error::other)?;
        let time_usage =
            mach_ticks_to_duration(r.ri_system_time) + mach_ticks_to_duration(r.ri_user_time);
        #[allow(clippy::cast_precision_loss)]
        let memory_usage = r.ri_lifetime_max_phys_footprint as f64 / MAXRSS_DIVISOR;
        let e = self
            .child
            .try_wait()
            .expect("If the first wait succeeds, the second wait cannot fail.")
            .expect("If the first wait succeeds, the second wait cannot fail.");
        self.result = Some(ExitInformation {
            time_usage,
            memory_usage,
            status: strsignal(e.signal()),
        });
        Ok(())
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

impl Drop for RunnerChild {
    fn drop(&mut self) {
        self.source.cancel();
    }
}

fn mach_ticks_to_duration(ticks: u64) -> Duration {
    let (numer, denom) = timebase();
    let nanos = (u128::from(ticks) * u128::from(numer)) / u128::from(denom);
    Duration::from_nanos_u128(nanos)
}

fn timebase() -> (u32, u32) {
    let mut info = mach2::mach_time::mach_timebase_info { numer: 0, denom: 0 };
    unsafe { mach2::mach_time::mach_timebase_info(&raw mut info) };
    (info.numer, info.denom)
}

pub const MAXRSS_DIVISOR: f64 = 1024.0 * 1024.0;

fn strsignal(sig: Option<i32>) -> Cow<'static, str> {
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
