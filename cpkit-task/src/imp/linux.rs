use std::{
    io,
    mem::MaybeUninit,
    num::NonZeroI32,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    process::Command,
    time::{Duration, Instant},
};

use async_io::{Async, Timer};
use futures_util::{FutureExt, select};
use linux_raw_sys::general::{
    CLD_DUMPED, CLD_KILLED, P_PIDFD, WEXITED, WNOHANG, rusage, siginfo_t,
};
use rustix::process::{Pid, PidfdFlags, Signal, pidfd_open, pidfd_send_signal};
use rustix_libc_wrappers::process::SignalExt;
use syscalls::{Sysno::waitid as SYS_waitid, syscall5};
use tokio_util::sync::CancellationToken;

use crate::{
    status::PrintableStatus,
    task::{Task, TaskError, TaskResult, Usage},
};

fn spawn(task: &Task) -> io::Result<Async<OwnedFd>> {
    let child = Command::new(&task.executable).spawn()?;
    // 创建 Pidfd 失败则静默让程序继续运行。风险通常是可接受的。
    pidfd_open(Pid::from_child(&child), PidfdFlags::empty())
        .map_err(Into::into)
        .and_then(Async::new_nonblocking)
}

fn get_result(fd: BorrowedFd<'_>) -> io::Result<Option<Usage>> {
    let mut siginfo = MaybeUninit::<siginfo_t>::zeroed();
    let mut rusage = MaybeUninit::<rusage>::uninit();
    unsafe {
        syscall5(
            SYS_waitid,
            P_PIDFD as usize,
            fd.as_raw_fd().cast_unsigned() as usize,
            siginfo.as_mut_ptr() as usize,
            (WNOHANG | WEXITED) as usize,
            rusage.as_mut_ptr() as usize,
        )
    }?;
    // Safety: siginfo 已经预先清零，即使没有子进程退出 siginfo 也是合法 Rust 值。
    let si = unsafe { siginfo.assume_init().__bindgen_anon_1.__bindgen_anon_1 };
    let (pid, status) = unsafe { (si._sifields._sigchld._pid, si._sifields._sigchld._status) };
    if pid == 0 {
        return Ok(None);
    }
    let signal = if matches!(si.si_code.cast_unsigned(), CLD_KILLED | CLD_DUMPED) {
        Signal::from_raw(status).map_or_else(
            || {
                PrintableStatus::Unknown(
                    NonZeroI32::new(status).expect("CLD_* implies non-zero status"),
                )
            },
            PrintableStatus::Known,
        )
    } else {
        PrintableStatus::ExitCode(status)
    };
    let rusage = unsafe { rusage.assume_init() };
    let time = Duration::from_secs(rusage.ru_utime.tv_sec.cast_unsigned())
        + Duration::from_micros(rusage.ru_utime.tv_usec.cast_unsigned())
        + Duration::from_secs(rusage.ru_stime.tv_sec.cast_unsigned())
        + Duration::from_micros(rusage.ru_stime.tv_usec.cast_unsigned());
    #[allow(clippy::cast_precision_loss)]
    let memory = rusage.ru_maxrss as f64 / 1024.0;
    Ok(Some(Usage {
        time,
        memory,
        status: signal,
    }))
}

pub async fn run_task(task: Task, token: CancellationToken) -> Result<TaskResult, TaskError> {
    let fd = spawn(&task)?;
    let mut deadline = task.time_limit.map(|d| Instant::now() + d);
    loop {
        select! {
            res = fd.readable().fuse() => res?,
            _ = FutureExt::fuse(deadline.map_or_else(Timer::never, Timer::at)) => {
                let _ = pidfd_send_signal(fd.as_fd(), Signal::KILL);
                deadline = None;
            }
            () = token.cancelled().fuse() => {
                let _ = pidfd_send_signal(fd.as_fd(), Signal::KILL);
                deadline = None;
            }
        }
        if let Some(usage) = get_result(fd.as_fd())? {
            return Ok(TaskResult {
                task: task.id,
                usage,
            });
        }
    }
}
