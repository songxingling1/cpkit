#![allow(dead_code)]
use libc::{c_int, pid_t};
use std::{
    io,
    mem::MaybeUninit,
    os::unix::process::ExitStatusExt,
    process::{ChildStderr, ChildStdin, ChildStdout},
};
use tokio::{
    signal::unix::{Signal, SignalKind, signal},
    time::sleep_until,
};

/// 将 libc 内函数返回值转换成 Result
fn cvt<T: PartialEq + From<i8>>(res: T) -> io::Result<T> {
    if res == T::from(-1) {
        Err(io::Error::last_os_error())
    } else {
        Ok(res)
    }
}

/// 不断重试 f 直到不返回 Interrupted
fn cvt_r<T>(mut f: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    loop {
        match f() {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            other => break other,
        }
    }
}

/// 运行结果
#[derive(Clone, Copy, Debug)]
pub struct RunResult {
    rusage: libc::rusage,
    exit_status: std::process::ExitStatus,
}

#[cfg_attr(test, mockall::automock)]
trait Os {
    fn wait4(&self, pid: pid_t, options: c_int) -> io::Result<Option<(c_int, libc::rusage)>>;
    fn kill(&self, pid: pid_t, sig: c_int) -> io::Result<c_int>;
    fn waitpid(&self, pid: pid_t, options: c_int) -> io::Result<Option<c_int>>;
}

struct LibcOs;

impl Os for LibcOs {
    fn kill(&self, pid: pid_t, sig: c_int) -> io::Result<c_int> {
        cvt(unsafe { libc::kill(pid, sig) })
    }
    fn wait4(&self, pid: pid_t, options: c_int) -> io::Result<Option<(c_int, libc::rusage)>> {
        let mut status: c_int = 0;
        let mut rusage = MaybeUninit::<libc::rusage>::uninit();
        let ret = cvt(unsafe { libc::wait4(pid, &raw mut status, options, rusage.as_mut_ptr()) })?;
        if ret == 0 {
            Ok(None)
        } else {
            Ok(Some((status, unsafe { rusage.assume_init() })))
        }
    }
    fn waitpid(&self, pid: pid_t, options: c_int) -> io::Result<Option<c_int>> {
        let mut status: c_int = 0;
        let ret = cvt(unsafe { libc::waitpid(pid, &raw mut status, options) })?;
        if ret == 0 { Ok(None) } else { Ok(Some(status)) }
    }
}

#[cfg_attr(test, mockall::automock)]
trait ChildLike {
    fn pid(&self) -> pid_t;
    fn stdin(&mut self) -> Option<ChildStdin>;
    fn stdout(&mut self) -> Option<ChildStdout>;
    fn stderr(&mut self) -> Option<ChildStderr>;
}

impl ChildLike for std::process::Child {
    fn pid(&self) -> pid_t {
        self.id() as pid_t
    }
    fn stdin(&mut self) -> Option<ChildStdin> {
        self.stdin.take()
    }
    fn stderr(&mut self) -> Option<ChildStderr> {
        self.stderr.take()
    }
    fn stdout(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }
}

#[cfg_attr(test, mockall::automock)]
trait SignalSource {
    fn recv(&mut self) -> impl Future<Output = ()> + Send;
}

impl SignalSource for Signal {
    async fn recv(&mut self) -> () {
        self.recv().await.expect("Signal never return None");
    }
}

struct InnerRusageChild<O: Os, S: SignalSource> {
    pid: pid_t,
    result: Option<RunResult>,
    deadline: tokio::time::Instant,
    signal: S,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
    os: O,
}

impl<O: Os, S: SignalSource> InnerRusageChild<O, S> {
    pub fn new(
        mut child: impl ChildLike,
        limit: tokio::time::Duration,
        os: O,
        signal: S,
    ) -> InnerRusageChild<O, S> {
        InnerRusageChild {
            stderr: child.stderr(),
            stdin: child.stdin(),
            stdout: child.stdout(),
            deadline: tokio::time::Instant::now() + limit,
            pid: child.pid(),
            result: None,
            signal,
            os,
        }
    }
    pub fn try_wait(&mut self) -> io::Result<Option<RunResult>> {
        if let Some(r) = self.result {
            return Ok(Some(r));
        }
        let res = self.os.wait4(self.pid, libc::WNOHANG)?;
        if let Some((status, rusage)) = res {
            let res = RunResult {
                rusage,
                exit_status: std::process::ExitStatus::from_raw(status),
            };
            self.result = Some(res);
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
    pub async fn wait(&mut self) -> io::Result<RunResult> {
        if let Some(r) = self.result {
            return Ok(r);
        }
        if let Ok(Some(r)) = self.try_wait() {
            return Ok(r);
        }
        let sleep = sleep_until(self.deadline);
        tokio::pin!(sleep);
        'main: loop {
            tokio::select! {
                () = &mut sleep => {
                    cvt_r(|| self.os.kill(self.pid,libc::SIGKILL))?;
                    let Some((status, rusage)) = cvt_r(|| self.os.wait4(self.pid, 0))? else {
                        unreachable!("wait4 without WNOHANG always returns Some on success")
                    };
                    let res = RunResult {
                        rusage,
                        exit_status: std::process::ExitStatus::from_raw(status),
                    };
                    self.result = Some(res);
                    break Ok(res)
                }
                () = self.signal.recv() => {
                    loop {
                        match self.try_wait() {
                            Ok(Some(r)) => break 'main Ok(r),
                            Ok(None) => continue 'main,
                            Err(e) if e.kind() == io::ErrorKind::Interrupted => {},
                            Err(e) => break 'main Err(e)
                        }
                    }
                }
            }
        }
    }
}

impl<O: Os, S: SignalSource> Drop for InnerRusageChild<O, S> {
    fn drop(&mut self) {
        if self.result.is_some() {
            return;
        }
        let _ = cvt_r(|| self.os.kill(self.pid, libc::SIGKILL));
        let _ = cvt_r(|| self.os.waitpid(self.pid, 0));
    }
}

/// 支持限制运行挂钟时间（较为粗略）和获取 rusage 的 Child。在 Child 被 drop 时自动清理进程。
pub struct RusageChild(InnerRusageChild<LibcOs, Signal>);

impl RusageChild {
    pub fn new(
        child: std::process::Child,
        limit: tokio::time::Duration,
    ) -> io::Result<RusageChild> {
        Ok(RusageChild(InnerRusageChild::new(
            child,
            limit,
            LibcOs,
            signal(SignalKind::child())?,
        )))
    }
    pub fn try_wait(&mut self) -> io::Result<Option<RunResult>> {
        self.0.try_wait()
    }
    pub async fn wait(&mut self) -> io::Result<RunResult> {
        self.0.wait().await
    }
}

pub trait RusageExt {
    fn spawn_with_rusage(self, limit: tokio::time::Duration) -> io::Result<RusageChild>;
}

impl RusageExt for tokio::process::Command {
    fn spawn_with_rusage(mut self, limit: tokio::time::Duration) -> io::Result<RusageChild> {
        RusageChild::new(self.as_std_mut().spawn()?, limit)
    }
}

impl RusageExt for std::process::Command {
    fn spawn_with_rusage(mut self, limit: tokio::time::Duration) -> io::Result<RusageChild> {
        RusageChild::new(self.spawn()?, limit)
    }
}

#[cfg(test)]
#[path = "tests/test_runner.rs"]
mod tests;
