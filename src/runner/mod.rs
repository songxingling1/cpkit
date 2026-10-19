use std::io::Result;

mod model;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod linux_pidfd;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use model::ExitInformation;

#[cfg(target_os = "linux")]
use linux::RunnerChild as SignalChild;
#[cfg(target_os = "linux")]
use linux_pidfd::RunnerChild as PidfdChild;
#[cfg(target_os = "macos")]
pub use macos::RunnerChild;
#[cfg(target_os = "windows")]
pub use windows::RunnerChild;

#[cfg(target_os = "linux")]
pub enum RunnerChild {
    Signal(SignalChild),
    Pidfd(PidfdChild),
}

#[cfg(target_os = "linux")]
impl RunnerChild {
    pub fn new(child: tokio::process::Child) -> Result<RunnerChild> {
        match PidfdChild::new(child) {
            Ok(c) => Ok(RunnerChild::Pidfd(c)),
            Err((None, returned)) => Ok(RunnerChild::Signal(SignalChild::new(returned)?)),
            Err((Some(e), _)) => Err(e),
        }
    }
    /// 击杀进程（发送 Kill 信号）
    pub fn kill(&mut self) -> Result<()> {
        match self {
            RunnerChild::Pidfd(c) => c.kill(),
            RunnerChild::Signal(c) => c.kill(),
        }
    }
    /// 尝试等待进程退出
    #[allow(dead_code)]
    pub fn try_wait(&mut self) -> Result<Option<&ExitInformation>> {
        match self {
            RunnerChild::Pidfd(c) => c.try_wait(),
            RunnerChild::Signal(c) => c.try_wait(),
        }
    }
    // 等待进程退出，允许被取消
    pub async fn wait(&mut self) -> Result<&ExitInformation> {
        match self {
            RunnerChild::Pidfd(c) => c.wait().await,
            RunnerChild::Signal(c) => c.wait().await,
        }
    }
}

pub trait RunnerExt {
    fn spawn_with_runner(&mut self) -> Result<RunnerChild>;
}

impl RunnerExt for tokio::process::Command {
    fn spawn_with_runner(&mut self) -> Result<RunnerChild> {
        RunnerChild::new(self.spawn()?)
    }
}

#[cfg(test)]
#[path = "../tests/test_runner.rs"]
mod tests;
