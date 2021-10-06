use std::{
    collections::HashMap,
    os::unix::prelude::RawFd,
    task::{Context, Poll},
};
use tokio::io::{unix::AsyncFd, Interest};

#[derive(Debug)]
pub struct IoMap {
    inner: HashMap<RawFd, IoEntry>,
}

impl IoMap {
    pub(crate) fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn add(&mut self, fd: RawFd, io_type: IoType) -> std::io::Result<Option<IoEntry>> {
        let interest = io_type.clone().into();
        let async_fd = AsyncFd::with_interest(fd, interest)?;

        Ok(self.inner.insert(fd, (async_fd, io_type)))
    }

    pub fn del(&mut self, fd: RawFd) -> Option<IoEntry> {
        self.inner.remove(&fd)
    }

    pub fn dispatch(&mut self, base: *mut libevent_sys::event_base, cx: &mut Context) -> bool {
        let mut ready = false;

        for (fd, (async_fd, io_type)) in &mut self.inner {
            if io_type.is_read() {
                if let Poll::Ready(Ok(mut guard)) = async_fd.poll_read_ready(cx) {
                    unsafe {
                        libevent_sys::evmap_io_active_(base, *fd, libevent_sys::EV_READ as i16);
                    }
                    guard.clear_ready();
                    ready = true;
                }
            }

            if io_type.is_write() {
                if let Poll::Ready(Ok(mut guard)) = async_fd.poll_write_ready(cx) {
                    unsafe {
                        libevent_sys::evmap_io_active_(base, *fd, libevent_sys::EV_WRITE as i16);
                    }
                    guard.clear_ready();
                    ready = true;
                }
            }
        }

        ready
    }
}

pub type IoEntry = (AsyncFd<RawFd>, IoType);

#[derive(Clone, Debug)]
pub enum IoType {
    Read,
    ReadWrite,
    Write,
}

impl IoType {
    pub fn from_events(events: u32) -> Option<Self> {
        let is_read = events & libevent_sys::EV_READ != 0;
        let is_write = events & libevent_sys::EV_WRITE != 0;

        if is_read && is_write {
            Some(IoType::ReadWrite)
        } else if is_read {
            Some(IoType::Read)
        } else if is_write {
            Some(IoType::Write)
        } else {
            None
        }
    }

    pub fn is_read(&self) -> bool {
        match self {
            IoType::Read => true,
            IoType::ReadWrite => true,
            IoType::Write => false,
        }
    }

    pub fn is_write(&self) -> bool {
        match self {
            IoType::Read => false,
            IoType::ReadWrite => true,
            IoType::Write => true,
        }
    }
}

impl From<IoType> for Interest {
    fn from(io_type: IoType) -> Self {
        match io_type {
            IoType::Read => Interest::READABLE,
            IoType::Write => Interest::WRITABLE,
            IoType::ReadWrite => Interest::READABLE.add(Interest::WRITABLE),
        }
    }
}
