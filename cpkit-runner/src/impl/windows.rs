use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    ptr::null_mut,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use dashmap::Entry;
use windows::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    System::Threading::{
        CreateEventW, INFINITE, RegisterWaitForSingleObject, SetEvent, UnregisterWaitEx,
        WORKER_THREAD_FLAGS, WT_EXECUTEDEFAULT, WT_EXECUTEONLYONCE,
    },
};

use crate::reactor::{Poll, Wake};
struct ProcessData {
    token: usize,
    is_inqueue: AtomicBool,
    _callback: Box<dyn Fn() + Send + Sync>,
    wait_handle: HANDLE,
    generation: u64,
}
// Safety: Wait 句柄可以被安全的移动到另一个线程。
unsafe impl Send for ProcessData {}
// Safety: 外界没有办法通过 ProcessData 的引用做任何操作，
// 也就不会导致内存问题。
unsafe impl Sync for ProcessData {}
pub struct ProcessPool {
    data: Arc<dashmap::DashMap<usize, ProcessData>>,
    rx: crossbeam_channel::Receiver<(usize, u64)>,
    tx: crossbeam_channel::Sender<(usize, u64)>,
    next_gen: AtomicU64,
}
pub struct ProcessPollWaker {
    event: OwnedHandle,
}
impl ProcessPool {
    fn register_wait(
        &self,
        source: &OwnedHandle,
        token: usize,
        flags: WORKER_THREAD_FLAGS,
    ) -> std::io::Result<()> {
        let key = source.as_raw_handle() as usize;
        let data = self.data.clone();
        let tx = self.tx.clone();
        let generation = self.next_gen.fetch_add(1, Ordering::Relaxed);
        let cb = Box::new(move || {
            if let Some(inner) = data.get(&key)
                && inner.generation == generation
                && let Ok(_) = inner.is_inqueue.compare_exchange(
                    false,
                    true,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
            {
                let _ = tx.send((key, generation));
            }
        });
        let mut wait_handle = HANDLE(null_mut());
        let Entry::Vacant(entry) = self.data.entry(key) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Handle already exists.",
            ));
        };
        unsafe {
            RegisterWaitForSingleObject(
                &raw mut wait_handle,
                HANDLE(source.as_raw_handle()),
                Some(route_callback(&cb)),
                Some(Box::as_ptr(&cb).cast()),
                INFINITE,
                flags,
            )?;
        };
        entry.insert(ProcessData {
            token,
            is_inqueue: AtomicBool::new(false),
            _callback: cb,
            wait_handle,
            generation,
        });
        Ok(())
    }
}
impl Wake for ProcessPollWaker {
    type Poll = ProcessPool;
    fn new(registry: &Self::Poll, token: usize) -> std::io::Result<Self> {
        let rawhandle = unsafe { CreateEventW(None, false, false, None)? };
        let event = unsafe { OwnedHandle::from_raw_handle(rawhandle.0) };
        registry.register_wait(&event, token, WT_EXECUTEDEFAULT)?;
        Ok(ProcessPollWaker { event })
    }
    fn wake(&self) -> std::io::Result<()> {
        unsafe { SetEvent(HANDLE(self.event.as_raw_handle()))? };
        Ok(())
    }
}
impl Poll for ProcessPool {
    type Source = OwnedHandle;
    type Waker = ProcessPollWaker;
    fn new() -> std::io::Result<Self> {
        let (tx, rx) = crossbeam_channel::unbounded();
        Ok(ProcessPool {
            data: Arc::new(dashmap::DashMap::new()),
            tx,
            rx,
            next_gen: AtomicU64::new(0),
        })
    }
    fn register(&self, source: &Self::Source, token: usize) -> std::io::Result<()> {
        self.register_wait(source, token, WT_EXECUTEONLYONCE)
    }
    fn deregister(&self, source: &Self::Source) -> std::io::Result<()> {
        let key = source.as_raw_handle() as usize;
        let inner = self.data.get(&key).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Handle not found.")
        })?;
        let wait_handle = inner.wait_handle;
        drop(inner);
        unsafe {
            UnregisterWaitEx(wait_handle, Some(INVALID_HANDLE_VALUE))?;
        };
        self.data.remove(&key);
        Ok(())
    }
    fn poll(&mut self) -> std::io::Result<Vec<usize>> {
        Ok(
            vec![self.rx.recv().unwrap()] // 结构体本身保留了一个发送端，不可能出现 `Err`
                .into_iter()
                .chain(self.rx.try_iter())
                .take(self.rx.len() + 1)
                .filter_map(|t| {
                    if let Some(inner) = self.data.get(&t.0)
                        && inner.generation == t.1
                    {
                        inner.is_inqueue.store(false, Ordering::Release);
                        Some(inner.token)
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }
}
fn route_callback<F: Fn() + Send + Sync>(
    _: &F,
) -> unsafe extern "system" fn(*mut std::ffi::c_void, bool) {
    process_callback::<F>
}
unsafe extern "system" fn process_callback<F>(ptr: *mut std::ffi::c_void, _timer_fired: bool)
where
    F: Fn() + Send + Sync,
{
    let cb = unsafe { &*(ptr.cast::<F>()) };
    cb();
}
