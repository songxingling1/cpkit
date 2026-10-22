use std::os::fd::{AsFd, OwnedFd};

use rustix::{
    buffer::spare_capacity,
    event::kqueue::{
        Event, EventFilter, EventFlags, ProcessEvents, UserDefinedFlags, UserFlags, kevent, kqueue,
    },
    process::Pid,
};

use crate::reactor::{Poll, Wake};

pub struct KQueue {
    kq: OwnedFd,
    events: Vec<Event>,
}
pub struct KQueueWaker {
    kq: OwnedFd,
    event: Event,
}
impl Wake for KQueueWaker {
    type Poll = KQueue;
    fn new(registry: &Self::Poll, token: usize) -> std::io::Result<Self> {
        let kq = registry.kq.try_clone()?;
        let event = Event::new(
            EventFilter::User {
                ident: 1,
                flags: UserFlags::NOINPUT,
                user_flags: UserDefinedFlags::new(0),
            },
            EventFlags::ADD | EventFlags::ENABLE,
            token as _,
        );
        unsafe {
            kevent::<_, &mut [Event; 0]>(kq.as_fd(), &[event], &mut [], None)?;
        };
        Ok(KQueueWaker {
            kq,
            event: Event::new(
                EventFilter::User {
                    ident: 1,
                    flags: UserFlags::TRIGGER,
                    user_flags: UserDefinedFlags::new(0),
                },
                EventFlags::ADD | EventFlags::ENABLE,
                token as _,
            ),
        })
    }
    fn wake(&self) -> std::io::Result<()> {
        unsafe {
            kevent::<_, &mut [Event; 0]>(self.kq.as_fd(), &[self.event], &mut [], None)?;
        };
        Ok(())
    }
}
impl Poll for KQueue {
    type Source = Pid;
    type Waker = KQueueWaker;
    fn new() -> std::io::Result<Self> {
        let kq = kqueue()?;
        Ok(KQueue {
            kq,
            events: Vec::with_capacity(128),
        })
    }
    fn register(&self, source: &Self::Source, token: usize) -> std::io::Result<()> {
        let event = Event::new(
            EventFilter::Proc {
                pid: *source,
                flags: ProcessEvents::EXIT,
            },
            EventFlags::ADD | EventFlags::ENABLE,
            token as _,
        );
        // Safety: Pid 的有效性由 rustix 和调用者共同保证。
        unsafe {
            kevent::<_, &mut [Event; 0]>(self.kq.as_fd(), &[event], &mut [], None)?;
        };
        Ok(())
    }
    fn deregister(&self, source: &Self::Source) -> std::io::Result<()> {
        let event = Event::new(
            EventFilter::Proc {
                pid: *source,
                flags: ProcessEvents::EXIT,
            },
            EventFlags::DELETE,
            std::ptr::null_mut(),
        );
        // Safety: Pid 的有效性由 rustix 和调用者共同保证。
        unsafe {
            kevent::<_, &mut [Event; 0]>(self.kq.as_fd(), &[event], &mut [], None)?;
        };
        Ok(())
    }
    fn poll(&mut self) -> std::io::Result<Vec<usize>> {
        self.events.clear();
        // Safety: 没传入任何 Event。
        unsafe {
            kevent(self.kq.as_fd(), &[], spare_capacity(&mut self.events), None)?;
        };
        Ok(self.events.iter().map(|e| e.udata() as usize).collect())
    }
}
