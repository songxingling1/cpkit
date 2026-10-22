//! Reactor。
//!
//! 注意，所有的 Request 必须保证内部的 Source 在注册期间一直是有效的。
#![allow(dead_code)]
use std::{
    io::Result,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Release},
    },
};

use atomic_waker::AtomicWaker;
use slab::Slab;
use tokio::sync::oneshot;

pub trait Poll: Sized {
    type Source;
    type Waker: Wake<Poll = Self>;
    fn new() -> Result<Self>;
    fn register(&self, source: &Self::Source, token: usize) -> Result<()>;
    fn deregister(&self, source: &Self::Source) -> Result<()>;
    fn poll(&mut self) -> Result<Vec<usize>>;
}

const WAKE_TOKEN: usize = usize::MAX;

pub trait Wake: Sized {
    type Poll: Poll;
    fn new(registry: &Self::Poll, token: usize) -> Result<Self>;
    fn wake(&self) -> Result<()>;
}

enum RunnerRequest<S> {
    Register {
        shared: Arc<ScheduledRunner<S>>,
        sender: oneshot::Sender<Result<usize>>,
    },
    Deregister {
        token: usize,
    },
}

pub struct ScheduledRunner<S> {
    pub source: S,
    pub waker: AtomicWaker,
    pub is_ready: AtomicBool,
    pub is_shutdown: AtomicBool,
}

impl<S> ScheduledRunner<S> {
    fn new(source: S) -> Self {
        ScheduledRunner {
            source,
            waker: AtomicWaker::new(),
            is_ready: AtomicBool::new(false),
            is_shutdown: AtomicBool::new(false),
        }
    }
}

pub struct Registration<P: Poll> {
    token: usize,
    pub shared: Arc<ScheduledRunner<P::Source>>,
    handle: RunnerReactorHandle<P>,
}

impl<P: Poll> Drop for Registration<P> {
    fn drop(&mut self) {
        let _ = self.handle.deregister(self);
    }
}

pub struct RunnerReactor<P: Poll> {
    poll: P,
    slab: Slab<Arc<ScheduledRunner<P::Source>>>,
    request: crossbeam_channel::Receiver<RunnerRequest<P::Source>>,
}

pub struct RunnerReactorHandle<P: Poll> {
    waker: Arc<P::Waker>,
    request: crossbeam_channel::Sender<RunnerRequest<P::Source>>,
}

impl<P: Poll> Clone for RunnerReactorHandle<P> {
    fn clone(&self) -> Self {
        Self {
            waker: self.waker.clone(),
            request: self.request.clone(),
        }
    }
}

impl<P: Poll> RunnerReactor<P> {
    pub fn new() -> Result<(RunnerReactor<P>, RunnerReactorHandle<P>)> {
        let poll = P::new()?;
        let waker = P::Waker::new(&poll, WAKE_TOKEN)?;
        let (tx, rx) = crossbeam_channel::unbounded();
        Ok((
            RunnerReactor {
                poll,
                slab: Slab::new(),
                request: rx,
            },
            RunnerReactorHandle {
                waker: Arc::new(waker),
                request: tx,
            },
        ))
    }
    pub fn run(&mut self) -> Result<()> {
        loop {
            self.drain();
            let events = self.poll.poll()?;
            for token in events {
                if token == WAKE_TOKEN {
                    continue;
                }
                if let Some(shared) = self.slab.get(token) {
                    shared.is_ready.store(true, Release);
                    shared.waker.wake();
                }
            }
        }
    }
    fn drain(&mut self) {
        for item in self.request.try_iter() {
            match item {
                RunnerRequest::Register { shared, sender } => {
                    let entry = self.slab.vacant_entry();
                    let token = entry.key();
                    match self.poll.register(&shared.source, token) {
                        Ok(()) => {
                            entry.insert(shared);
                            let _ = sender.send(Ok(token));
                        }
                        Err(e) => {
                            let _ = sender.send(Err(e));
                        }
                    }
                }
                RunnerRequest::Deregister { token } => {
                    if let Some(shared) = self.slab.get(token) {
                        let _ = self.poll.deregister(&shared.source);
                        self.slab.remove(token);
                    }
                }
            }
        }
    }
}

impl<P: Poll> Drop for RunnerReactor<P> {
    fn drop(&mut self) {
        for item in self.slab.drain() {
            item.is_shutdown.store(true, Release);
            item.waker.wake();
        }
    }
}

impl<P: Poll> RunnerReactorHandle<P> {
    pub(crate) async fn register(&self, source: P::Source) -> Result<Registration<P>> {
        let (tx, rx) = oneshot::channel();
        let shared = Arc::new(ScheduledRunner::new(source));
        self.request
            .send(RunnerRequest::Register {
                shared: shared.clone(),
                sender: tx,
            })
            .map_err(|_| std::io::Error::other("reactor closed"))?;
        self.waker.wake()?;
        let token = rx
            .await
            .map_err(|_| std::io::Error::other("reactor closed"))??;
        Ok(Registration {
            token,
            shared,
            handle: self.clone(),
        })
    }
    fn deregister(&self, registration: &Registration<P>) -> Result<()> {
        self.request
            .send(RunnerRequest::Deregister {
                token: registration.token,
            })
            .map_err(|_| std::io::Error::other("reactor closed"))?;
        self.waker.wake()?;
        Ok(())
    }
}
