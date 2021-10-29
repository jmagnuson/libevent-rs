use super::{
    io::{IoMap, IoType},
    runtime::Runtime,
    signal::SignalMap,
    BaseWrapper,
};
use crate::tokio_backend::runtime::{JoinFuture, YieldFuture};
use std::{os::raw::c_int, sync::Arc, time::Duration};
use tokio::{sync::Notify, task::JoinHandle};

/// Implements a libevent backend using a tokio runtime
pub struct TokioBackend {
    /// Option tokio runtime to maintain ownership
    runtime: Box<dyn Runtime>,
    /// Notifies the dispatch loop that it should yield back to libevent
    dispatch_notify: Arc<Notify>,
    /// Map of futures for registered I/O events
    io_map: IoMap,
    /// Map of futures for registered signals
    signal_map: SignalMap,
}

impl TokioBackend {
    /// Create a new libevent backend using the provided runtime
    pub fn new(runtime: Box<dyn Runtime>) -> Self {
        let dispatch_notify = Arc::new(Notify::new());
        let io_map = IoMap::new();
        let signal_map = SignalMap::new();

        Self {
            runtime,
            dispatch_notify,
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
    pub(crate) fn add_io(&mut self, base: BaseWrapper, fd: c_int, io_type: IoType) -> c_int {
        tracing::debug!(fd, ?io_type, "add an I/O event");

        let _guard = self.runtime.enter();

        match self
            .io_map
            .add(base, fd, io_type, self.dispatch_notify.clone())
        {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!(?error);
                -1
            }
        }
    }

    /// Blocks on the given join handle
    fn join(&self, join_handle: JoinHandle<()>) {
        let future = JoinFuture::new(join_handle);

        self.runtime.join(future);
    }

    /// Terminates an active I/O task
    pub fn del_io(&mut self, fd: c_int, io_type: IoType) -> c_int {
        tracing::debug!(fd, ?io_type, "delete an I/O event");

        if let Ok(result) = self.io_map.del(fd, io_type) {
            if let Some(join_handle) = result {
                self.join(join_handle);
            }

            0
        } else {
            -1
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
    pub(crate) fn add_signal(&mut self, base: BaseWrapper, signum: c_int) -> c_int {
        tracing::debug!(signum, "add a signal event");

        let _guard = self.runtime.enter();

        match self
            .signal_map
            .add(base, signum, self.dispatch_notify.clone())
        {
            Ok(_) => 0,
            Err(error) => {
                tracing::error!(?error);
                -1
            }
        }
    }

    /// Terminates an active signal task
    pub fn del_signal(&mut self, signum: c_int) -> c_int {
        tracing::debug!(signum, "delete a signal event");

        if let Ok(join_handle) = self.signal_map.del(signum) {
            self.join(join_handle);
            0
        } else {
            -1
        }
    }

    /// Drive the tokio runtime with an optional duration for timout events
    pub fn dispatch(&mut self, _base: *mut libevent_sys::event_base, timeout: Option<Duration>) {
        tracing::trace!(?timeout, "dispatch events");

        let _guard = self.runtime.enter();

        match timeout {
            Some(duration) => {
                if duration.is_zero() {
                    let future = YieldFuture::default();

                    self.runtime.dispatch_yield(future);
                } else {
                    let future = self.dispatch_notify.notified();
                    let future = tokio::time::timeout(duration, future);

                    self.runtime.dispatch_timeout(future);
                }
            }
            None => {
                let future = self.dispatch_notify.notified();

                self.runtime.dispatch_notify(future);
            }
        }
    }
}
