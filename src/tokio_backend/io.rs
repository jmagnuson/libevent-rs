use super::BaseWrapper;
use std::{
    collections::HashMap,
    os::unix::prelude::*,
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    },
};
use tokio::{
    io::{unix::AsyncFd, Interest},
    sync::Notify,
    task::JoinHandle,
};

/// Manages adding and removing I/O event tasks
#[derive(Debug)]
pub struct IoMap {
    inner: HashMap<RawFd, IoEntry>,
}

#[derive(Clone, Debug)]
pub enum IoType {
    Read,
    ReadWrite,
    Write,
}

#[derive(Debug)]
struct IoContext {
    notify: Notify,
    nread: AtomicI32,
    nwrite: AtomicI32,
}

#[derive(Debug)]
struct IoEntry {
    context: Arc<IoContext>,
    join_handle: JoinHandle<()>,
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
        match self.inner.get(&fd) {
            Some(entry) => {
                if io_type.is_read() {
                    entry.context.nread.fetch_add(1, Ordering::AcqRel);
                }

                if io_type.is_write() {
                    entry.context.nwrite.fetch_add(1, Ordering::AcqRel);
                }

                entry.context.notify.notify_one();
            }
            None => {
                let context = Arc::new(IoContext {
                    notify: Notify::new(),
                    nread: AtomicI32::new(if io_type.is_read() { 1 } else { 0 }),
                    nwrite: AtomicI32::new(if io_type.is_write() { 1 } else { 0 }),
                });
                let async_fd = AsyncFd::new(fd)?;
                let join_handle = tokio::spawn(io_task(
                    async_fd,
                    base,
                    context.clone(),
                    dispatch_notify.clone(),
                ));
                let entry = IoEntry {
                    context,
                    join_handle,
                };

                self.inner.insert(fd, entry);
            }
        }

        Ok(())
    }

    pub fn del(&mut self, fd: RawFd, io_type: IoType) -> Result<Option<JoinHandle<()>>, ()> {
        let total = {
            let entry = self.inner.get_mut(&fd).ok_or_else(|| ())?;

            let nread = if io_type.is_read() {
                entry.context.nread.fetch_sub(1, Ordering::AcqRel)
            } else {
                entry.context.nread.load(Ordering::Acquire)
            };
            assert!(nread >= 0);

            let nwrite = if io_type.is_write() {
                entry.context.nwrite.fetch_sub(1, Ordering::AcqRel)
            } else {
                entry.context.nread.load(Ordering::Acquire)
            };
            assert!(nwrite >= 0);

            entry.context.notify.notify_one();

            nread + nwrite
        };

        Ok(if total > 0 {
            let entry = self.inner.remove(&fd).unwrap();

            Some(entry.join_handle)
        } else {
            None
        })
    }
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

async fn io_task(
    async_fd: AsyncFd<RawFd>,
    base: BaseWrapper,
    context: Arc<IoContext>,
    dispatch_notify: Arc<Notify>,
) {
    let fd = async_fd.as_raw_fd();

    loop {
        tokio::select! {
            _ = context.notify.notified() => {
                let total = context.nread.load(Ordering::Acquire) + context.nwrite.load(Ordering::Acquire);

                if total == 0 {
                    tracing::debug!(fd, "I/O task removed");
                    return;
                }
            },
            result = async_fd.readable(), if context.nread.load(Ordering::Acquire) > 0 => {
                if let Ok(mut guard) = result {
                    unsafe {
                        libevent_sys::evmap_io_active_(base.0, fd, libevent_sys::EV_READ as i16);
                    }
                    guard.clear_ready();
                    dispatch_notify.notify_one();
                }
            },
            result = async_fd.writable(), if context.nwrite.load(Ordering::Acquire) > 0 => {
                if let Ok(mut guard) = result {
                    unsafe {
                        libevent_sys::evmap_io_active_(base.0, fd, libevent_sys::EV_WRITE as i16);
                    }
                    guard.clear_ready();
                    dispatch_notify.notify_one();
                }
            },
        }
    }
}
