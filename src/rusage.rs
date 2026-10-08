#![allow(dead_code)]
use libc::{c_int, pid_t, rusage};
use std::io::Result;
use std::mem::MaybeUninit;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ExitStatus};
use tokio::signal::unix::{Signal, SignalKind, signal};

/// 退出信息，包含退出状态和 Rusage
#[derive(Debug, Clone, Copy)]
pub struct ExitInformation {
    exit_status: ExitStatus,
    rusage: rusage,
}

#[cfg_attr(test, mockall::automock)]
trait Os {
    fn wait4(&self, pid: pid_t, options: c_int) -> Result<Option<ExitInformation>>;
    async fn recv_child(&mut self) -> ();
}

#[cfg_attr(test, mockall::automock)]
trait ChildLike {
    fn id(&self) -> pid_t;
    fn kill(&mut self) -> Result<()>;
}

struct InnerRusageChild<O: Os, C: ChildLike> {
    os: O,
    result: Option<ExitInformation>,
    child: C,
}

impl<O: Os, C: ChildLike> InnerRusageChild<O, C> {
    pub fn kill(&mut self) -> Result<()> {
        if self.result.is_some() {
            Ok(())
        } else {
            self.child.kill()
        }
    }
    pub fn try_wait(&mut self) -> Result<Option<ExitInformation>> {
        if let Some(res) = self.result {
            return Ok(Some(res));
        }
        if let Some(res) = self.os.wait4(self.child.id(), libc::WNOHANG)? {
            self.result = Some(res);
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
    pub async fn wait(&mut self) -> Result<ExitInformation> {
        if let Some(res) = self.result {
            return Ok(res);
        }
        if let Some(res) = self.try_wait()? {
            return Ok(res);
        }
        loop {
            self.os.recv_child().await;
            if let Some(res) = self.try_wait()? {
                break Ok(res);
            }
        }
    }
}

struct UnixOs {
    signal: Signal,
}

impl UnixOs {
    fn new() -> Result<UnixOs> {
        Ok(UnixOs {
            signal: signal(SignalKind::child())?,
        })
    }
    fn cvt(pid: pid_t) -> Result<pid_t> {
        if pid == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(pid)
        }
    }
}

impl Os for UnixOs {
    fn wait4(&self, pid: pid_t, options: c_int) -> Result<Option<ExitInformation>> {
        let mut status: c_int = 0;
        let mut rusage = MaybeUninit::<rusage>::uninit();
        // SAFETY: status 和 rusage 均为有效的指针，所以 wait4 是安全的。
        let res =
            Self::cvt(unsafe { libc::wait4(pid, &raw mut status, options, rusage.as_mut_ptr()) })?;
        if res == 0 {
            Ok(None)
        } else {
            Ok(Some(ExitInformation {
                exit_status: ExitStatus::from_raw(status),
                // SAFETY: status 和 rusage 都不是空指针并且 wait4 已经正确返回，所以此时初始化是安全的
                rusage: unsafe { rusage.assume_init() },
            }))
        }
    }
    async fn recv_child(&mut self) -> () {
        self.signal.recv().await.expect("Signal never return None");
    }
}

impl ChildLike for Child {
    fn id(&self) -> pid_t {
        self.id() as pid_t
    }
    fn kill(&mut self) -> Result<()> {
        self.kill()
    }
}

/// 可以获得 Rusage 信息的 Child
pub struct RusageChild(InnerRusageChild<UnixOs, Child>);

impl RusageChild {
    pub fn new(child: std::process::Child) -> Result<RusageChild> {
        Ok(RusageChild(InnerRusageChild {
            os: UnixOs::new()?,
            result: None,
            child,
        }))
    }
    pub fn try_wait(&mut self) -> Result<Option<ExitInformation>> {
        self.0.try_wait()
    }
    pub async fn wait(&mut self) -> Result<ExitInformation> {
        self.0.wait().await
    }
    pub fn kill(&mut self) -> Result<()> {
        self.0.kill()
    }
}

pub trait RusageExt {
    fn spawn_with_rusage(&mut self) -> Result<RusageChild>;
}

impl RusageExt for std::process::Command {
    fn spawn_with_rusage(&mut self) -> Result<RusageChild> {
        RusageChild::new(self.spawn()?)
    }
}

#[cfg(test)]
#[path = "tests/test_rusage.rs"]
mod tests;
