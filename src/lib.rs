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

pub struct Libevent {
    base: Base,
}

impl Libevent {
    pub fn new() -> Result<Self, io::Error> {
        Base::new().map(|base| Libevent { base })
    }

    pub unsafe fn from_raw(base: *mut libevent_sys::event_base) -> Result<Self, io::Error> {
        let base = Base::from_raw(base)?;
        Ok(Libevent { base })
    }

    // TODO: This should be raw_base, and EventBase should prevent having to use raw altogether.
    /// # Safety
    /// Exposes the event_base handle, which can be used to make any sort of
    /// modifications to the event loop without going through proper checks.
    pub unsafe fn with_base<F>(&mut self, f: F) -> c_int
    where
        F: Fn(*mut libevent_sys::event_base) -> c_int,
    {
        f(self.base.as_inner_mut())
    }

    /// # Safety
    /// Exposes the event_base handle, which can be used to make any sort of
    /// modifications to the event loop without going through proper checks.
    pub unsafe fn base(&self) -> &Base {
        &self.base
    }

    /// # Safety
    /// Exposes the event_base handle, which can be used to make any sort of
    /// modifications to the event loop without going through proper checks.
    pub unsafe fn base_mut(&mut self) -> &mut Base {
        &mut self.base
    }

    /// Turns the libevent base once.
    // TODO: any way to show if work was done?
    pub fn turn(&self) -> ExitReason {
        self.base.loop_(LoopFlags::NONBLOCK)
    }

    /// Turns the libevent base until exit or timeout duration reached.
    pub fn run_timeout(&self, timeout: Duration) -> ExitReason {
        if self.base.loopexit(timeout) != 0 {
            // TODO: This conflates errors, is it ok?
            return ExitReason::Error;
        };
        self.base.loop_(LoopFlags::empty())
    }

    /// Turns the libevent base until next active event.
    pub fn run_until_event(&self, timeout: Option<Duration>) -> ExitReason {
        if let Some(timeout) = timeout {
            if self.base.loopexit(timeout) != 0 {
                // TODO: This conflates errors, is it ok?
                return ExitReason::Error;
            }
        }
        self.base.loop_(LoopFlags::ONCE)
    }

    /// Turns the libevent base until exit.
    pub fn run(&self) -> ExitReason {
        self.base.loop_(LoopFlags::empty())
    }

    fn add_timer<F>(&mut self, tv: Duration, cb: F, flags: EventFlags) -> io::Result<EventHandle>
    where
        F: FnMut(&mut EventHandle, EventFlags) + 'static,
    {
        // First allocate the event with no context, then apply the reference
        // to the closure (and itself) later on.
        let mut ev = unsafe {
            self.base_mut()
                .event_new(None, flags, handle_wrapped_callback, None)
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
            self.base_mut().event_assign(
                &mut ev,
                None,
                flags,
                handle_wrapped_callback,
                Some(std::mem::transmute(cb_wrapped)),
            )
        };

        let _ = unsafe { self.base().event_add(&ev, Some(tv)) };

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
            self.base_mut().event_new(
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
            self.base_mut().event_assign(
                &mut ev,
                Some(fd),
                EventFlags::PERSIST | EventFlags::READ,
                handle_wrapped_callback,
                Some(std::mem::transmute(cb_wrapped)),
            )
        };

        let _ = unsafe { self.base().event_add(&ev, tv) };

        Ok(ev)
    }
}
