use super::BaseWrapper;
use std::{collections::HashMap, os::unix::prelude::RawFd, sync::Arc};
use tokio::{
    io::{unix::AsyncFd, Interest},
    sync::Notify,
    task::JoinHandle,
};

/// Manages adding and removing I/O event tasks
#[derive(Debug)]
pub struct IoMap {
    inner: HashMap<RawFd, (Arc<Notify>, JoinHandle<()>)>,
}

impl IoMap {
    pub(crate) fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn add(
        &mut self,
        base: BaseWrapper,
        fd: RawFd,
        io_type: IoType,
        dispatch_notify: Arc<Notify>,
    ) -> std::io::Result<()> {
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let interest = io_type.clone().into();
        let async_fd = AsyncFd::with_interest(fd, interest)?;
        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = async_fd.readable(), if io_type.is_read() => {
                        if let Ok(mut guard) = result {
                            unsafe {
                                libevent_sys::evmap_io_active_(base.0, fd, libevent_sys::EV_READ as i16);
                            }
                            guard.clear_ready();
                            dispatch_notify.notify_one();
                        }
                    },
                    result = async_fd.writable(), if io_type.is_write() => {
                        if let Ok(mut guard) = result {
                            unsafe {
                                libevent_sys::evmap_io_active_(base.0, fd, libevent_sys::EV_WRITE as i16);
                            }
                            guard.clear_ready();
                            dispatch_notify.notify_one();
                        }
                    },
                    _ = notify.notified() => {
                        break;
                    }
                }
            }

            tracing::debug!(fd, ?io_type, "I/O task removed");
        });

        self.inner.insert(fd, (notify_clone, join_handle));

        Ok(())
    }

    pub fn del(&mut self, fd: RawFd) -> Result<JoinHandle<()>, ()> {
        if let Some((notify, join_handle)) = self.inner.remove(&fd) {
            notify.notify_one();
            Ok(join_handle)
        } else {
            Err(())
        }
    }
}

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
