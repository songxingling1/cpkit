use std::{
    io,
    mem::MaybeUninit,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle},
    process::{Child, Command},
    time::{Duration, Instant},
};

use async_io::{Timer, os::windows::Waitable};
use futures_util::{FutureExt, select};
use tokio_util::sync::CancellationToken;
use windows::Win32::{
    Foundation::{FILETIME, HANDLE},
    System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetExitCodeProcess, GetProcessTimes},
    },
};

use crate::{
    Task, TaskResult,
    status::PrintableStatus,
    task::{TaskError, Usage},
};

fn spawn(task: &Task) -> io::Result<Waitable<Child>> {
    let child = Command::new(&task.executable).spawn()?;
    Waitable::new(child)
}
fn get_process_memory_info(handle: BorrowedHandle<'_>) -> io::Result<PROCESS_MEMORY_COUNTERS> {
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

fn get_process_run_time(handle: BorrowedHandle<'_>) -> io::Result<Duration> {
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

fn get_process_exit_code(handle: BorrowedHandle<'_>) -> io::Result<u32> {
    let mut res = 0u32;
    unsafe { GetExitCodeProcess(HANDLE(handle.as_raw_handle()), &raw mut res) }?;
    Ok(res)
}

fn get_result(handle: BorrowedHandle<'_>) -> io::Result<Usage> {
    let memory = get_process_memory_info(handle.as_handle())?;
    let time = get_process_run_time(handle.as_handle())?;
    #[allow(clippy::cast_precision_loss)]
    let memory = (memory.PeakWorkingSetSize as f64) / (1024.0 * 1024.0);
    let exitcode = get_process_exit_code(handle.as_handle())?;
    Ok(Usage {
        time,
        memory,
        status: PrintableStatus(exitcode),
    })
}

pub async fn run_task(task: Task, token: CancellationToken) -> Result<TaskResult, TaskError> {
    let mut process = spawn(&task)?;
    let deadline = task.time_limit.map(|d| Instant::now() + d);
    loop {
        select! {
            res = process.ready().fuse() => {
                res?;
                break
            },
            _ = FutureExt::fuse(deadline.map_or_else(Timer::never, Timer::at)) => {
                // Safety：kill 不会把 Child move / drop。
                let _ = unsafe { process.get_mut().kill() };
            }
            () = token.cancelled().fuse() => {
                let _ = unsafe { process.get_mut().kill() };
            }
        }
    }
    return Ok(TaskResult {
        task: task.id,
        usage: get_result(process.as_handle())?,
    });
}
