#![allow(dead_code)]

use libevent_sys;
use std::io;
use std::os::raw::{c_int, c_short};
#[cfg(unix)]
use std::os::unix::io::RawFd;
use std::time::Duration;

mod base;
pub use base::*;
mod event;
pub use event::*;

/// Gets used as the boxed context for `ExternCallbackFn`
struct EventCallbackWrapper {
    inner: Box<dyn FnMut(&mut EventHandle, EventFlags)>,
    ev: EventHandle,
}

extern "C" fn handle_wrapped_callback(_fd: EvutilSocket, event: c_short, ctx: EventCallbackCtx) {
    let cb_ref = unsafe {
        let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
        let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
        _cb_ref
    };

    let flags = EventFlags::from_bits_truncate(event as u32);
    let event_handle = &mut cb_ref.ev;
    (cb_ref.inner)(event_handle, flags)
}

impl Base {
    /// Turns the libevent base once.
    // TODO: any way to show if work was done?
    pub fn turn(&self) -> ExitReason {
        self.loop_(LoopFlags::NONBLOCK)
    }

    /// Turns the libevent base until exit or timeout duration reached.
    pub fn run_timeout(&self, timeout: Duration) -> ExitReason {
        if self.loopexit(timeout) != 0 {
            // TODO: This conflates errors, is it ok?
            return ExitReason::Error;
        };
        self.loop_(LoopFlags::empty())
    }

    /// Turns the libevent base until next active event.
    pub fn run_until_event(&self, timeout: Option<Duration>) -> ExitReason {
        if let Some(timeout) = timeout {
            if self.loopexit(timeout) != 0 {
                // TODO: This conflates errors, is it ok?
                return ExitReason::Error;
            }
        }
        self.loop_(LoopFlags::ONCE)
    }

    /// Turns the libevent base until exit.
    pub fn run(&self) -> ExitReason {
        self.loop_(LoopFlags::empty())
    }

    fn add_timer<F>(&mut self, tv: Duration, cb: F, flags: EventFlags) -> io::Result<EventHandle>
    where
        F: FnMut(&mut EventHandle, EventFlags) + 'static,
    {
        // First allocate the event with no context, then apply the reference
        // to the closure (and itself) later on.
        let mut ev = unsafe { self.event_new(None, flags, handle_wrapped_callback, None) };

        unsafe {
            // A gross way to signify that we're leaking the boxed
            // `EventCallbackWrapper` match libevent's context type.
            // TODO: Use `event_finalize` to de-init boxed closure.
            ev.inner.lock().unwrap().set_drop_ctx();
        }

        let cb_wrapped = Box::new(EventCallbackWrapper {
            inner: Box::new(cb),
            ev: ev.clone(),
        });

        // Now we can apply the closure + handle to self.
        let _ = unsafe {
            self.event_assign(
                &mut ev,
                None,
                flags,
                handle_wrapped_callback,
                Some(std::mem::transmute(cb_wrapped)),
            )
        };

        let _ = unsafe { self.event_add(&ev, Some(tv)) };

        Ok(ev)
    }

    pub fn add_interval<F>(&mut self, interval: Duration, cb: F) -> io::Result<EventHandle>
    where
        F: FnMut(&mut EventHandle, EventFlags) + 'static,
    {
        self.add_timer(interval, cb, EventFlags::PERSIST)
    }

    pub fn add_oneshot<F>(&mut self, timeout: Duration, cb: F) -> io::Result<EventHandle>
    where
        F: FnMut(&mut EventHandle, EventFlags) + 'static,
    {
        self.add_timer(timeout, cb, EventFlags::empty())
    }

    #[cfg(unix)]
    pub fn add_fd<F>(&mut self, fd: RawFd, tv: Option<Duration>, cb: F) -> io::Result<EventHandle>
    where
        F: FnMut(&mut EventHandle, EventFlags) + 'static,
    {
        // First allocate the event with no context, then apply the reference
        // to the closure (and itself) later on.
        let mut ev = unsafe {
            self.event_new(
                Some(fd),
                EventFlags::PERSIST | EventFlags::READ,
                handle_wrapped_callback,
                None,
            )
        };

        unsafe {
            // A gross way to signify that we're leaking the boxed
            // `EventCallbackWrapper` match libevent's context type.
            // TODO: Use `event_finalize` to de-init boxed closure.
            ev.inner.lock().unwrap().set_drop_ctx();
        }

        let cb_wrapped = Box::new(EventCallbackWrapper {
            inner: Box::new(cb),
            ev: ev.clone(),
        });

        // Now we can apply the closure + handle to self.
        let _ = unsafe {
            self.event_assign(
                &mut ev,
                Some(fd),
                EventFlags::PERSIST | EventFlags::READ,
                handle_wrapped_callback,
                Some(std::mem::transmute(cb_wrapped)),
            )
        };

        let _ = unsafe { self.event_add(&ev, tv) };

        Ok(ev)
    }
}
