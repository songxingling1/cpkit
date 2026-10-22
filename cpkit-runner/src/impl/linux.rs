use std::os::fd::{AsFd, OwnedFd};

use crate::reactor::{Poll, Wake};

use rustix::{
    buffer::spare_capacity,
    event::{
        EventfdFlags,
        epoll::{self, CreateFlags, Event, EventData, EventFlags},
        eventfd,
    },
    io::write,
};
pub struct Epoll {
    ep: OwnedFd,
    event: Vec<Event>,
}
pub struct EpollWaker {
    ev: OwnedFd,
}
impl Wake for EpollWaker {
    type Poll = Epoll;
    fn new(registry: &Self::Poll, token: usize) -> std::io::Result<Self> {
        let eventfd = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)?;
        registry.register(&eventfd, token)?;
        Ok(EpollWaker { ev: eventfd })
    }
    fn wake(&self) -> std::io::Result<()> {
        write(self.ev.as_fd(), &[1])?;
        Ok(())
    }
}
impl Poll for Epoll {
    type Source = OwnedFd;
    type Waker = EpollWaker;
    fn new() -> std::io::Result<Self> {
        let fd = epoll::create(CreateFlags::CLOEXEC)?;
        Ok(Epoll {
            ep: fd,
            event: Vec::with_capacity(128),
        })
    }
    fn register(&self, source: &Self::Source, token: usize) -> std::io::Result<()> {
        Ok(epoll::add(
            self.ep.as_fd(),
            source,
            EventData::new_u64(token as u64),
            EventFlags::IN | EventFlags::ET,
        )?)
    }
    fn deregister(&self, source: &Self::Source) -> std::io::Result<()> {
        Ok(epoll::delete(self.ep.as_fd(), source)?)
    }
    fn poll(&mut self) -> std::io::Result<Vec<usize>> {
        self.event.clear();
        epoll::wait(self.ep.as_fd(), spare_capacity(&mut self.event), None)?;
        Ok(self
            .event
            .iter()
            .map(|e| usize::try_from(e.data.u64()).unwrap()) // 一开始传进去的就是 usize，转回来自然不会出现溢出。
            .collect())
    }
}
