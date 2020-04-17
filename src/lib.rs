#![allow(dead_code)]

use bitflags::bitflags;
use std::io;
use std::os::raw::{c_int, c_short, c_void};
use std::time::Duration;
use libevent_sys;

type EvutilSocket = c_int;

type EventCallbackFn = extern "C" fn(EvutilSocket, c_short, *mut c_void);

/// Gets used as the boxed context for `EXternCallbackFn`
struct EventCallbackWrapper {
    inner: Box<dyn FnMut()>,
}

extern "C" fn handle_wrapped_callback(fd: EvutilSocket, event: c_short, ctx: *mut c_void) {
    let cb_ref = unsafe {
        let cb: *mut EventCallbackWrapper = std::mem::transmute( ctx );
        let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
        _cb_ref
    };

    (cb_ref.inner)()
}


fn to_timeval(duration: Duration) -> libevent_sys::timeval {
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "solaris"))]
        let tv = libevent_sys::timeval {
        tv_sec: duration.as_secs() as libevent_sys::__time_t,
        tv_usec: duration.subsec_micros() as libevent_sys::__suseconds_t,
    };

    #[cfg(any(target_os = "bitrig", target_os = "dragonfly",
    target_os = "freebsd", target_os = "ios", target_os = "macos",
    target_os = "netbsd", target_os = "openbsd"))]
        let tv = libevent_sys::timeval {
        tv_sec: duration.as_secs() as libevent_sys::time_t,
        tv_usec: duration.subsec_micros() as libevent_sys::suseconds_t,
    };

    tv
}

pub struct EventBase {
    base: *mut libevent_sys::event_base
}

unsafe impl Send for EventBase {}
unsafe impl Sync for EventBase {}

/// The handle that abstracts over libevent's API in Rust.
impl EventBase {
    pub fn new() -> Result<Self, io::Error> {
        let base = unsafe {
            libevent_sys::event_base_new()
        };

        if base.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to create libevent base"));
        }

        Ok(EventBase {
            base,
        })
    }

    pub fn as_inner(&self) -> *const libevent_sys::event_base {
        self.base as *const libevent_sys::event_base
    }

    pub fn as_inner_mut(&self) -> *mut libevent_sys::event_base {
        self.base
    }

    pub fn loop_(&self, flags: LoopFlags) -> ExitReason {
        let exit_code = unsafe {
            libevent_sys::event_base_loop(self.base, flags.bits() as i32) as i32
        };

        match exit_code {
            0 => {
                unsafe {
                    // Technically mutually-exclusive from `got_break`, but
                    // the check in `event_base_loop` comes first, so the logic
                    // here matches.
                    if libevent_sys::event_base_got_exit(self.base) != 0i32 {
                        ExitReason::GotExit
                    }
                    else if libevent_sys::event_base_got_break(self.base) != 0i32 {
                        ExitReason::GotBreak
                    } else {
                        // TODO: This should match flags for `EVLOOP_ONCE`, `_NONBLOCK`, etc.
                        ExitReason::Unknown{ flags, exit_code }
                    }
                }
            },
            -1 => ExitReason::Error,
            1 => ExitReason::NoPendingEvents,
            _ => ExitReason::Unknown{flags, exit_code}
        }
    }

    pub fn loopexit(&self, timeout: Duration) -> i32 {
        let tv = to_timeval(timeout);
        unsafe {
            let tv_cast = &tv as *const libevent_sys::timeval;
            libevent_sys::event_base_loopexit(self.base, tv_cast) as i32
        }
    }

    pub fn loopbreak(&self) -> i32 {
        unsafe {
            libevent_sys::event_base_loopbreak(self.base) as i32
        }
    }

    pub fn loopcontinue(&self) -> i32 {
        unsafe {
            libevent_sys::event_base_loopcontinue(self.base) as i32
        }
    }

    pub fn event_new(
        &mut self,
        fd: Option<EvutilSocket>,
        flags: c_short,
        callback: EventCallbackFn,
        callback_ctx: *mut c_void,
    ) -> *mut libevent_sys::event {
        let fd: EvutilSocket = if let Some(fd_) = fd {
            // Actual fd
            fd_
        } else {
            // Timer
            -1
        };

        unsafe {
            libevent_sys::event_new(
                self.as_inner_mut(),
                fd,
                flags,
                Some(callback),
                callback_ctx,
            )
        }
    }

    pub fn event_add(
        //&mut self,
        & self,
        event: *mut libevent_sys::event,
        timeout: Duration,
    ) -> c_int {
        let tv = to_timeval(timeout);
        unsafe {
            libevent_sys::event_add(event, &tv)
        }
    }
}

pub struct Libevent {
    base: EventBase,
}

impl Libevent {
    pub fn new() -> Result<Self, io::Error> {
        EventBase::new()
            .map(|base| Libevent { base })
    }

    // TODO: This should be raw_base, and EventBase should prevent having to use raw altogether.
    pub unsafe fn with_base<F: Fn(*mut libevent_sys::event_base) -> c_int>(
        &self,
        f: F
    ) -> c_int
        where
    {
        f(self.base.as_inner_mut())
    }

    pub/*(crate)*/ unsafe fn base(&self) -> &EventBase {
        &self.base
    }

    pub/*(crate)*/ unsafe fn base_mut(&mut self) -> &mut EventBase {
        &mut self.base
    }

    /// Turns the libevent base once.
    // TODO: any way to show if work was done?
    pub fn turn(&self) -> bool {
        let _retval = self.base.loop_(LoopFlags::NONBLOCK);

        true
    }

    /// Turns the libevent base until exit or timeout duration reached.
    // TODO: any way to show if work was done?
    pub fn run_timeout(&self, timeout: Duration) -> bool {
        let _retval = self.base.loopexit(timeout);
        let _retval = self.base.loop_(LoopFlags::empty());

        true
    }

    /// Turns the libevent base until next active event.
    // TODO: any way to show if work was done?
    pub fn run_until_event(&self) -> bool {
        let _retval = self.base.loop_(LoopFlags::ONCE);

        true
    }

    /// Turns the libevent base until exit.
    // TODO: any way to show if work was done?
    pub fn run(&self) -> bool {
        let _retval = self.base.loop_(LoopFlags::empty());

        true
    }

    pub fn add_interval<F: FnMut() + 'static>(&mut self, interval: Duration, cb: F) -> io::Result<EventHandle> {
        let cb_wrapped = Box::new(EventCallbackWrapper {
            inner: Box::new(cb)
        });

        let ev = unsafe { self.base_mut().event_new(
            None,
            libevent_sys::EV_PERSIST as c_short,
            handle_wrapped_callback,
            unsafe {std::mem::transmute(cb_wrapped) },
        ) };

        let _ = unsafe {
            self.base().event_add(ev, interval)
        };

        Ok(EventHandle { inner: ev })
    }
}

pub struct EventHandle {
    inner: *mut libevent_sys::event,
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        unsafe { libevent_sys::event_free(self.inner) }
    }
}

pub enum ExitReason {
    GotExit,
    GotBreak,
    Error,
    NoPendingEvents,
    Unknown{ flags: LoopFlags, exit_code: i32 },
}

bitflags! {
    pub struct LoopFlags: u32 {
        const ONCE = libevent_sys::EVLOOP_ONCE;
        const NONBLOCK = libevent_sys::EVLOOP_NONBLOCK;
        const NO_EXIT_ON_EMPTY = libevent_sys::EVLOOP_NO_EXIT_ON_EMPTY;
    }
}