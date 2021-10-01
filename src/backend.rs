use std::{
    collections::HashMap,
    ffi::c_void,
    os::raw::{c_int, c_short},
    ptr::NonNull,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::unix::AsyncFd,
    signal::unix::{signal, SignalKind},
    sync::Notify,
};
use tracing::instrument;

/// Implements a libevent backend using a tokio runtime
#[derive(Debug)]
pub struct TokioBackend {
    /// Callback functions and configuration for I/O handling
    evsel: libevent_sys::eventop,
    /// Callback functions and configuration for signal handling
    evsigsel: libevent_sys::eventop,
    /// Notifies the dispatch loop that it should yield back to libevent
    dispatch_notify: Arc<Notify>,
    /// Tokio runtime for driving I/O and signal events
    runtime: tokio::runtime::Runtime,
    /// Map of active signals to task shutdown notifications
    signal_map: HashMap<c_int, Arc<Notify>>,
}

impl TokioBackend {
    /// Create a new tokio libevent backend using the provided runtime
    #[instrument]
    fn new(runtime: tokio::runtime::Runtime) -> Self {
        const EVSEL: libevent_sys::eventop = libevent_sys::eventop {
            name: "tokio".as_ptr().cast(),
            init: Some(tokio_backend_init),
            add: Some(tokio_backend_add),
            del: Some(tokio_backend_del),
            dispatch: Some(tokio_backend_dispatch),
            dealloc: Some(tokio_backend_dealloc),
            need_reinit: 1,
            features: libevent_sys::event_method_feature_EV_FEATURE_FDS,
            fdinfo_len: std::mem::size_of::<Arc<Notify>>() as u64,
        };
        const EVSIGSEL: libevent_sys::eventop = libevent_sys::eventop {
            name: "tokio_signal".as_ptr().cast(),
            init: None,
            add: Some(tokio_signal_backend_add),
            del: Some(tokio_signal_backend_del),
            dispatch: None,
            dealloc: None,
            need_reinit: 0,
            features: 0,
            fdinfo_len: 0,
        };
        let dispatch_notify = Arc::new(Notify::new());
        let signal_map = HashMap::new();

        Self {
            evsel: EVSEL,
            evsigsel: EVSIGSEL,
            dispatch_notify,
            runtime,
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
    #[instrument(skip(base))]
    fn add_io(
        &self,
        base: *mut libevent_sys::event_base,
        fd: c_int,
        events: c_short,
        fdinfo: *mut Arc<Notify>,
    ) -> c_int {
        tracing::debug!("add an I/O event");

        // signal events should never be passed to this function
        assert_eq!(event_is_signal(events), false);

        let base = BaseWrapper(base as *mut libevent_sys::event_base);
        let is_read = event_is_read(events);
        let is_write = event_is_write(events);
        let _guard = self.runtime.enter();
        let dispatch_notify = self.dispatch_notify.clone();

        match AsyncFd::new(fd) {
            Ok(async_fd) => {
                // The Arc<Notify> is cloned and copied into the allocated space
                // for the file descriptor. The object is no longer guarded and
                // must be dropped manually when the I/O event is deleted.
                let notify = Arc::new(Notify::new());
                unsafe {
                    fdinfo.write(notify.clone());
                }

                // A tokio task is spawned to service each I/O event.
                self.runtime.spawn(async move {
                    loop {
                        tokio::select! {
                            result = async_fd.readable(), if is_read => {
                                match result {
                                    Ok(mut guard) => {
                                        tracing::debug!(fd, "I/O ready to read");
                                        unsafe {
                                            // libevent dispatches the callback mapped to the file descriptor.
                                            libevent_sys::evmap_io_active_(base.0, fd, libevent_sys::EV_READ as i16);
                                        }

                                        // If the ready flag is not cleared, then this loop will hang the runtime.
                                        guard.clear_ready();
                                        dispatch_notify.notify_one();
                                    }
                                    Err(error) => {
                                        tracing::error!(?error);
                                        break;
                                    }
                                }
                            },
                            result = async_fd.writable(), if is_write => {
                                match result {
                                    Ok(mut guard) => {
                                        tracing::debug!(fd, "I/O ready to write");
                                        unsafe {
                                            // libevent dispatches the callback with the mapped file descriptor
                                            libevent_sys::evmap_io_active_(base.0, fd, libevent_sys::EV_WRITE as i16);
                                        }

                                        // If the ready flag is not cleared, then this loop will hang the runtime.
                                        guard.clear_ready();
                                        dispatch_notify.notify_one();
                                    }
                                    Err(error) => {
                                        tracing::error!(?error);
                                        break;
                                    }
                                }
                            },
                            _ = notify.notified() => break, // terminate the task on notification
                        }
                    }

                    tracing::debug!(?fd, "I/O task terminated");
                });

                0
            }
            Err(error) => {
                tracing::error!(?error);
                -1
            }
        }
    }

    /// Terminates an active I/O task
    #[instrument]
    fn del_io(&self, fdinfo: *mut Arc<Notify>) -> c_int {
        tracing::debug!("delete an I/O event");

        unsafe {
            match fdinfo.as_mut() {
                Some(notify) => {
                    notify.notify_one();
                    std::ptr::drop_in_place(fdinfo);
                    0
                }
                None => -1,
            }
        }
    }

    /// Drive the tokio runtime with an optional duration for timout events
    #[instrument]
    fn dispatch(&self, timeout: Option<Duration>) {
        let notify = self.dispatch_notify.clone();

        self.runtime.block_on(async move {
            match timeout {
                // spawned tasks are serviced during the sleep time
                Some(timeout) => {
                    tokio::select! {
                        _ = tokio::time::sleep(timeout) => (),
                        _ = notify.notified() => (),
                    }
                }
                // at least a single yield is required to advance any pending tasks
                None => tokio::task::yield_now().await,
            }
        })
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
    #[instrument(skip(base))]
    fn add_signal(
        &mut self,
        base: *mut libevent_sys::event_base,
        nsignal: c_int,
        events: c_short,
    ) -> c_int {
        let base = BaseWrapper(base);

        tracing::debug!("add a signal event");

        // I/O events should never be passed to this function
        assert!(event_is_signal(events));

        let _guard = self.runtime.enter();
        let dispatch_notify = self.dispatch_notify.clone();

        match signal(SignalKind::from_raw(nsignal)) {
            Ok(mut stream) => {
                // map a shutdown notification to the signal number
                let notify = Arc::new(Notify::new());
                if let Some(old_notify) = self.signal_map.insert(nsignal, notify.clone()) {
                    /*
                     * This should not happend since libevent tracks signal
                     * registration. However, any previous task should be
                     * shutdown to ensure that it does not leak.
                     */
                    old_notify.notify_one();
                }

                // a tokio task is spawned per signal number
                self.runtime.spawn(async move {
                    loop {
                        tokio::select! {
                            result = stream.recv() => {
                                if result.is_some() {
                                    tracing::debug!(nsignal, "signal received");
                                    unsafe {
                                        // libevent dispatches callbacks with the mapped signal
                                        libevent_sys::evmap_signal_active_(base.0, nsignal, 1);
                                    }
                                    dispatch_notify.notify_one();
                                } else {
                                    tracing::error!("signal stream has closed");
                                    break;
                                }
                            },
                            _ = notify.notified() => break, // terminate the task on notification
                        }
                    }

                    // indicate that the task has terminated
                    tracing::debug!(?nsignal, "I/O task terminated");
                });

                0
            }
            Err(error) => {
                tracing::error!(?error);
                -1
            }
        }
    }

    /// Terminates an active signal task
    #[instrument]
    fn del_signal(&mut self, nsignal: c_int) -> c_int {
        tracing::debug!("delete an signal event");

        match self.signal_map.remove(&nsignal) {
            Some(notify) => {
                notify.notify_one();
                0
            }
            None => {
                tracing::warn!("signal not found");
                -1
            }
        }
    }
}

/// Wrapper to allow sending of raw event_base pointers to tokio tasks.
///
/// This is safe because libevent performs locking internally.
struct BaseWrapper(pub *mut libevent_sys::event_base);

unsafe impl Send for BaseWrapper {}

/// Injects a tokio backend with the given runtime into the given libevent instance.
///
/// The libevent instance will already have an initialized backend. This
/// exisiting backend is deallocated before being replaced.
///
/// A tracing-subscriber may also be initialized if the feature is activated
/// to enable tracing output when linked to a C program.
pub fn inject_tokio(mut base: NonNull<libevent_sys::event_base>, runtime: tokio::runtime::Runtime) {
    #[cfg(feature = "tracing_subscriber")]
    tracing_subscriber::fmt::init();

    let backend = Box::new(TokioBackend::new(runtime));
    let base = unsafe { base.as_mut() };

    if let Some(evsel) = unsafe { base.evsel.as_ref() } {
        if let Some(dealloc) = evsel.dealloc {
            unsafe {
                dealloc(base);
            }
        }
    }

    base.evsel = &backend.evsel;
    base.evsigsel = &backend.evsigsel;
    base.evbase = Box::into_raw(backend).cast();
}

/// Convenience function that returns true if the signal event bit is set.
fn event_is_signal(events: c_short) -> bool {
    let events = events as u32;

    events & libevent_sys::EV_SIGNAL != 0
}

/// Convenience function that returns true if the I/O read event bit is set.
fn event_is_read(events: c_short) -> bool {
    let events = events as u32;

    events & libevent_sys::EV_READ != 0
}

/// Convenience function that returns true if the I/O write event bit is set.
fn event_is_write(events: c_short) -> bool {
    let events = events as u32;

    events & libevent_sys::EV_WRITE != 0
}

/// Convenience method to allow injecting C programs with a tokio backend
#[no_mangle]
pub unsafe extern "C" fn tokio_event_base_new() -> *mut libevent_sys::event_base {
    let base = NonNull::new(libevent_sys::event_base_new());

    match base {
        Some(base) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build a tokio runtime");

            inject_tokio(base, runtime);

            base.as_ptr()
        }
        None => std::ptr::null_mut(),
    }
}

/// libevent callback to initialize the backend
///
/// This function would normally be called in `event_base_new`, but the tokio
/// backend is inject after that call. Therefore, this call would only happen
/// if the process is forked. That functionality is not currently supported.
#[no_mangle]
pub unsafe extern "C" fn tokio_backend_init(_base: *mut libevent_sys::event_base) -> *mut c_void {
    unimplemented!("forking with a tokio backend")
}

/// libevent callback to add an I/O event
#[no_mangle]
pub unsafe extern "C" fn tokio_backend_add(
    eb: *mut libevent_sys::event_base,
    fd: c_int,
    _old: c_short,
    events: c_short,
    fdinfo: *mut c_void,
) -> c_int {
    if let Some(base) = eb.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_ref() {
            return backend.add_io(eb, fd, events, fdinfo.cast());
        }
    }

    -1
}

/// libevent callback to remove an I/O event
#[no_mangle]
unsafe extern "C" fn tokio_backend_del(
    base: *mut libevent_sys::event_base,
    _fd: c_int,
    _old: c_short,
    _events: c_short,
    fdinfo: *mut c_void,
) -> c_int {
    if let Some(base) = base.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_ref() {
            return backend.del_io(fdinfo.cast());
        }
    }

    -1
}

/// libevent callback to drive the event loop
#[no_mangle]
unsafe extern "C" fn tokio_backend_dispatch(
    base: *mut libevent_sys::event_base,
    tv: *mut libevent_sys::timeval,
) -> c_int {
    if let Some(base) = base.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_ref() {
            let timeout = tv.as_ref().map(|tv| {
                Duration::from_secs(tv.tv_sec as u64)
                    .saturating_add(Duration::from_micros(tv.tv_usec as u64))
            });

            backend.dispatch(timeout);

            return 0;
        }
    }

    -1
}

/// libevent callback to deallocate the backend
#[no_mangle]
pub unsafe extern "C" fn tokio_backend_dealloc(base: *mut libevent_sys::event_base) {
    if let Some(base) = base.as_mut() {
        Box::from_raw(base.evbase);
        base.evbase = std::ptr::null_mut();
    }
}

/// libevent callback to add a signal event
#[no_mangle]
pub unsafe extern "C" fn tokio_signal_backend_add(
    eb: *mut libevent_sys::event_base,
    fd: c_int,
    _old: c_short,
    events: c_short,
    _fdinfo: *mut c_void,
) -> c_int {
    if let Some(base) = eb.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_mut() {
            return backend.add_signal(eb, fd, events);
        }
    }

    -1
}

/// libevent callback to remove a signal event
#[no_mangle]
unsafe extern "C" fn tokio_signal_backend_del(
    base: *mut libevent_sys::event_base,
    fd: c_int,
    _old: c_short,
    _events: c_short,
    _fdinfo: *mut c_void,
) -> c_int {
    if let Some(base) = base.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_mut() {
            return backend.del_signal(fd);
        }
    }

    -1
}
