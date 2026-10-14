use super::model::ExitInformation;
use std::io::Result;
use std::mem::MaybeUninit;
use std::os::windows::io::{
    AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle, RawHandle,
};
use std::time::Duration;
use windows::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE,
};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

/// 运行器（使用 Signal 实现）
pub struct RunnerChild {
    child: tokio::process::Child,
    result: Option<ExitInformation>,
    handle: OwnedHandle,
}

impl RunnerChild {
    pub fn new(child: tokio::process::Child) -> Result<RunnerChild> {
        let handle: RawHandle = child.raw_handle().ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "The child process shouldn't be reaped.",
        ))?;
        // SAFETY：我们刚刚获取了 Child 的句柄，由于此时我们并没有调用任何会改变内部状态的函数，
        // 并且 Rust 的所有权转移保证了外部不会有人调用改变内部状态的函数，
        // 所以这里是安全的。
        let handle = unsafe { BorrowedHandle::borrow_raw(handle) };
        Ok(RunnerChild {
            child,
            result: None,
            handle: duplicate_handle(handle)?,
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
    #[allow(dead_code)]
    pub fn try_wait(&mut self) -> Result<Option<&ExitInformation>> {
        if self.result.is_none()
            && let Some(res) = self.child.try_wait()?
        {
            let memory = get_process_memory_info(self.handle.as_handle())?;
            let time = get_process_run_time(self.handle.as_handle())?;
            // 非常显然，一个程序的峰值内存很难超过 4096 TB。
            #[allow(clippy::cast_precision_loss)]
            let memory = (memory.PeakWorkingSetSize as f64) / (1024.0 * 1024.0);
            self.result = Some(ExitInformation {
                time_usage: time,
                memory_usage: memory,
                status: std::borrow::Cow::Owned(res.to_string()),
            });
        }
        Ok(self.result.as_ref())
    }
    // 等待进程退出，允许被取消
    pub async fn wait(&mut self) -> Result<&ExitInformation> {
        if self.result.is_none() {
            let res = self.child.wait().await?;
            let memory = get_process_memory_info(self.handle.as_handle())?;
            let time = get_process_run_time(self.handle.as_handle())?;
            // 非常显然，一个程序的峰值内存很难超过 4096 TB。
            #[allow(clippy::cast_precision_loss)]
            let memory = (memory.PeakWorkingSetSize as f64) / (1024.0 * 1024.0);
            self.result = Some(ExitInformation {
                time_usage: time,
                memory_usage: memory,
                status: std::borrow::Cow::Owned(res.to_string()),
            });
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

fn duplicate_handle(handle: BorrowedHandle<'_>) -> Result<OwnedHandle> {
    let mut dup: HANDLE = INVALID_HANDLE_VALUE;
    // SAFETY: GetCurrentProcess 返回有效句柄，有复制权限
    // handle 是一个调用者传过来的句柄，由于 BorrowedHandle 所以是有效的
    // dup 是有效的栈上指针
    // 标志位合法
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(handle.as_raw_handle()),
            GetCurrentProcess(),
            &raw mut dup,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    }?;
    // SAFETY: 因为错误已经用 ? 抛出，所以这里的句柄一定有效，并且没有其他所有者。
    Ok(unsafe { OwnedHandle::from_raw_handle(dup.0) })
}

fn get_process_memory_info(handle: BorrowedHandle<'_>) -> Result<PROCESS_MEMORY_COUNTERS> {
    let mut counters = MaybeUninit::<PROCESS_MEMORY_COUNTERS>::uninit();
    // SAFETY: handle 是一个 BorrowedHandle，编译器保证合法
    // counters 是 MaybeUninit，显然是合法的指针并且有一块合法的内存。
    unsafe {
        GetProcessMemoryInfo(
            HANDLE(handle.as_raw_handle()),
            counters.as_mut_ptr(),
            u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>())
                .expect("The size of PROCESS_MEMORY_COUNTERS cannot overflow a u32."),
        )
    }?;
    // SAFETY：在上一行中，已经调用 GetProcessMemoryInfo 获取内存使用，
    // 并且错误情况已经被抛出，
    // 所以此时这里面是有效数据。
    Ok(unsafe { counters.assume_init() })
}

fn get_process_run_time(handle: BorrowedHandle<'_>) -> Result<Duration> {
    let mut creationtime = MaybeUninit::<FILETIME>::uninit();
    let mut exittime = MaybeUninit::<FILETIME>::uninit();
    let mut kerneltime = MaybeUninit::<FILETIME>::uninit();
    let mut usertime = MaybeUninit::<FILETIME>::uninit();
    // SAFETY: 句柄由 BorrowedHandle 保证合法
    // 四个时间指针均由上面四行用 MaybeUninit 创建，所以安全。
    unsafe {
        GetProcessTimes(
            HANDLE(handle.as_raw_handle()),
            creationtime.as_mut_ptr(),
            exittime.as_mut_ptr(),
            kerneltime.as_mut_ptr(),
            usertime.as_mut_ptr(),
        )
    }?;
    // SAFETY: 函数已经成功返回，并且 kerneltime 和 usertime 不同于 exittime，
    // 其值永远不可能是 undefined，所以是安全的。
    let kerneltime = unsafe { kerneltime.assume_init() };
    let usertime = unsafe { usertime.assume_init() };
    let kerneltick =
        (u128::from(kerneltime.dwHighDateTime) << 32) | u128::from(kerneltime.dwLowDateTime);
    let usertick = (u128::from(usertime.dwHighDateTime) << 32) | u128::from(usertime.dwLowDateTime);
    Ok(Duration::from_nanos_u128(kerneltick * 100) + Duration::from_nanos_u128(usertick * 100))
}
