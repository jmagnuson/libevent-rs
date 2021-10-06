use super::{
    io::{IoMap, IoType},
    signal::SignalMap,
};
use std::{
    future::Future,
    os::raw::c_int,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tracing::instrument;

/// Implements a libevent backend using a tokio runtime
#[derive(Debug)]
pub struct TokioBackend {
    /// Tokio runtime for driving I/O and signal events
    runtime: tokio::runtime::Runtime,
    /// Local set for running libevent callbacks on a single thread
    local: tokio::task::LocalSet,
    /// Map of futures for registered I/O events
    io_map: IoMap,
    /// Map of futures for registered signals
    signal_map: SignalMap,
}

impl TokioBackend {
    /// Create a new tokio libevent backend using the provided runtime
    #[instrument]
    pub fn new(runtime: tokio::runtime::Runtime) -> Self {
        let local = tokio::task::LocalSet::new();
        let io_map = IoMap::new();
        let signal_map = SignalMap::new();

        Self {
            runtime,
            local,
            io_map,
            signal_map,
        }
    }

    /// Creates a task to service a libevent I/O request
    ///
    /// A task must continue to service the file descriptor events until
    /// explicitly removed. Space is allocated by libevent that is used
    /// to store an Arc<Notify> object for clean shutdown of the created
    /// task.
    ///
    /// AsyncFd is used to assess read and write readiness of the
    /// file descriptor. All higher level funcitonality like socket listening
    /// and DNS request rely on these readiness notifications, but they
    /// otherwise function using unchanged libevent code.
    #[instrument]
    pub fn add_io(&mut self, fd: c_int, io_type: IoType) -> c_int {
        tracing::debug!("add an I/O event");

        let _guard = self.runtime.enter();

        match self.io_map.add(fd, io_type) {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!(?error);
                -1
            }
        }
    }

    /// Terminates an active I/O task
    #[instrument]
    pub fn del_io(&mut self, fd: c_int) -> c_int {
        tracing::debug!("delete an I/O event");

        match self.io_map.del(fd) {
            Some(_) => 0,
            None => -1,
        }
    }

    /// Creates a task to service a libevent signal request
    ///
    /// A task must continue to provide signal notifications until explicitly
    /// removed. Note that libevent does not provide user data per signal
    /// event. Therefore, signals are mapped to notifications in TokioBackend
    /// to allow for clean task shutdown.
    ///
    /// Since the tokio signal handler is installed globally. It is safe to
    /// handle signals with both libevent and using tokio directly.
    #[instrument]
    pub fn add_signal(&mut self, signum: c_int) -> c_int {
        tracing::debug!("add a signal event");

        let _guard = self.runtime.enter();

        match self.signal_map.add(signum) {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!(?error);
                -1
            }
        }
    }

    /// Terminates an active signal task
    #[instrument]
    pub fn del_signal(&mut self, signum: c_int) -> c_int {
        tracing::debug!("delete an signal event");

        match self.signal_map.del(signum) {
            Some(_) => 0,
            None => -1,
        }
    }

    /// Drive the tokio runtime with an optional duration for timout events
    #[instrument]
    pub fn dispatch(&mut self, base: *mut libevent_sys::event_base, timeout: Option<Duration>) {
        let future = Dispatcher {
            base,
            io_map: Pin::new(&mut self.io_map),
            signal_map: Pin::new(&mut self.signal_map),
        };

        self.local.block_on(&self.runtime, async move {
            if let Some(duration) = timeout {
                let _ = tokio::time::timeout(duration, future).await;
            } else {
                future.await
            }
        })
    }
}

struct Dispatcher<'a> {
    base: *mut libevent_sys::event_base,
    io_map: Pin<&'a mut IoMap>,
    signal_map: Pin<&'a mut SignalMap>,
}

impl<'a> Future for Dispatcher<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let project = self.get_mut();
        let base = project.base;
        let flag = project.io_map.dispatch(base, cx);
        let flag = project.signal_map.dispatch(base, cx) || flag;

        if flag {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
