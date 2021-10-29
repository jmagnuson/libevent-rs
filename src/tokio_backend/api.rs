use super::{
    backend::TokioBackend,
    io::IoType,
    runtime::{Runtime, TokioRuntime},
    BaseWrapper,
};
use libevent_sys::size_t;
use std::{
    ffi::c_void,
    os::{
        raw::{c_int, c_short},
        unix::io::RawFd,
    },
    ptr::NonNull,
    time::Duration,
};

const EVSEL: libevent_sys::eventop = libevent_sys::eventop {
    name: "tokio".as_ptr().cast(),
    init: Some(tokio_backend_init),
    add: Some(tokio_backend_add),
    del: Some(tokio_backend_del),
    dispatch: Some(tokio_backend_dispatch),
    dealloc: Some(tokio_backend_dealloc),
    need_reinit: 1,
    features: libevent_sys::event_method_feature_EV_FEATURE_FDS,
    fdinfo_len: std::mem::size_of::<RawFd>() as size_t,
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

/// Injects a tokio backend with the given runtime into the given libevent instance.
///
/// The libevent instance will already have an initialized backend. This
/// exisiting backend is deallocated before being replaced.
pub fn inject_tokio(mut base: NonNull<libevent_sys::event_base>, runtime: Box<dyn Runtime>) {
    let backend = Box::new(TokioBackend::new(runtime));
    let base = unsafe { base.as_mut() };

    if let Some(evsel) = unsafe { base.evsel.as_ref() } {
        if let Some(dealloc) = evsel.dealloc {
            unsafe {
                dealloc(base);
            }
        }
    }

    base.evsel = &EVSEL;
    base.evsigsel = &EVSIGSEL;
    base.evbase = Box::into_raw(backend).cast();
}

/// Convenience method to allow injecting C programs with a tokio backend
#[no_mangle]
pub unsafe extern "C" fn tokio_event_base_new() -> *mut libevent_sys::event_base {
    let base = NonNull::new(libevent_sys::event_base_new());

    match base {
        Some(base) => match TokioRuntime::new() {
            Ok(runtime) => {
                inject_tokio(base, Box::new(runtime));

                base.as_ptr()
            }
            Err(error) => {
                tracing::error!(?error, "failed to create a new Tokio runtime");

                std::ptr::null_mut()
            }
        },
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
    _fdinfo: *mut c_void,
) -> c_int {
    if let Some(io_type) = IoType::from_events(events as u32) {
        if let Some(base) = eb.as_ref() {
            if let Some(backend) = (base.evbase as *mut TokioBackend).as_mut() {
                return backend.add_io(BaseWrapper(eb), fd, io_type);
            }
        }
    }

    -1
}

/// libevent callback to remove an I/O event
#[no_mangle]
unsafe extern "C" fn tokio_backend_del(
    base: *mut libevent_sys::event_base,
    fd: c_int,
    _old: c_short,
    events: c_short,
    _fdinfo: *mut c_void,
) -> c_int {
    if let Some(base) = base.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_mut() {
            if let Some(io_type) = IoType::from_events(events as u32) {
                return backend.del_io(fd, io_type);
            }
        }
    }

    -1
}

/// libevent callback to drive the event loop
#[no_mangle]
unsafe extern "C" fn tokio_backend_dispatch(
    eb: *mut libevent_sys::event_base,
    tv: *mut libevent_sys::timeval,
) -> c_int {
    if let Some(base) = eb.as_ref() {
        if let Some(backend) = (base.evbase as *mut TokioBackend).as_mut() {
            let timeout = tv.as_ref().map(|tv| {
                Duration::from_secs(tv.tv_sec as u64)
                    .saturating_add(Duration::from_micros(tv.tv_usec as u64))
            });

            backend.dispatch(eb, timeout);

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
    signum: c_int,
    _old: c_short,
    events: c_short,
    _fdinfo: *mut c_void,
) -> c_int {
    if events as u32 & libevent_sys::EV_SIGNAL != 0 {
        if let Some(base) = eb.as_ref() {
            if let Some(backend) = (base.evbase as *mut TokioBackend).as_mut() {
                return backend.add_signal(BaseWrapper(eb), signum);
            }
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
