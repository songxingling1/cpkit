use super::{
    model::ExitInformation,
    unix::{MAXRSS_DIVISOR, strsignal, wait4},
};
use std::io::Result;
use std::{os::unix::process::ExitStatusExt, time::Duration};
use tokio::signal::unix::{Signal, SignalKind, signal};

/// 运行器（使用 Signal 实现）
pub struct RunnerChild {
    child: std::process::Child,
    result: Option<ExitInformation>,
    signal: Signal,
}

impl RunnerChild {
    pub fn new(child: std::process::Child) -> Result<RunnerChild> {
        Ok(RunnerChild {
            child,
            result: None,
            signal: signal(SignalKind::child())?,
        })
    }
    /// 击杀进程（发送 Kill 信号）
    pub fn kill(&mut self) -> Result<()> {
        if self.result.is_some() {
            Ok(())
        } else {
            self.child.kill()
        }
    }
    /// 尝试等待进程退出
    pub fn try_wait(&mut self) -> Result<Option<&ExitInformation>> {
        if self.result.is_none()
            && let Some((e, r)) = wait4(self.child.id() as libc::pid_t, libc::WNOHANG)?
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
                self.signal.recv().await;
            }
        }
        Ok(self.result.as_ref().unwrap())
    }
    #[allow(dead_code)]
    pub fn stdin(&self) -> &Option<std::process::ChildStdin> {
        &self.child.stdin
    }
    #[allow(dead_code)]
    pub fn stdin_mut(&mut self) -> &mut Option<std::process::ChildStdin> {
        &mut self.child.stdin
    }
    #[allow(dead_code)]
    pub fn stdout(&self) -> &Option<std::process::ChildStdout> {
        &self.child.stdout
    }
    #[allow(dead_code)]
    pub fn stdout_mut(&mut self) -> &mut Option<std::process::ChildStdout> {
        &mut self.child.stdout
    }
    #[allow(dead_code)]
    pub fn stderr(&self) -> &Option<std::process::ChildStderr> {
        &self.child.stderr
    }
    #[allow(dead_code)]
    pub fn stderr_mut(&mut self) -> &mut Option<std::process::ChildStderr> {
        &mut self.child.stderr
    }
}
