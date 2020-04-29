#![allow(dead_code)]

use bitflags::bitflags;
use std::io;
use std::os::raw::{c_int, c_short, c_void};
use std::time::Duration;
use libevent_sys;

use crate::to_timeval;

use super::event::*;
use std::sync::Arc;

pub type EvutilSocket = c_int;

pub type EventCallbackFn = extern "C" fn(EvutilSocket, EventCallbackFlags, EventCallbackCtx);
pub type EventCallbackCtx = *mut c_void;
pub type EventCallbackFlags = c_short;

pub struct EventBase {
    base: *mut libevent_sys::event_base
}

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

    pub fn as_inner_mut(&mut self) -> *mut libevent_sys::event_base {
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
        flags: EventFlags,
        callback: EventCallbackFn,
        callback_ctx: Option<EventCallbackCtx>,
    ) -> EventHandle {
        let fd: EvutilSocket = if let Some(fd_) = fd {
            // Actual fd
            fd_
        } else {
            // Timer
            -1
        };

        let callback_ctx = if let Some(_ctx) = callback_ctx {
            _ctx
        } else {
            std::ptr::null::<c_void>() as *mut std::ffi::c_void
        };

        let inner = unsafe {
            libevent_sys::event_new(
                self.as_inner_mut(),
                fd,
                flags.bits() as c_short,
                Some(callback),
                callback_ctx,
            )
        };

        EventHandle { inner: Arc::new(EventHandleInner { inner } ) }
    }

    pub fn event_assign(
        &mut self,
        ev: &mut EventHandle,
        fd: Option<EvutilSocket>,
        flags: EventFlags,
        callback: EventCallbackFn,
        callback_ctx: Option<EventCallbackCtx>,
    ) -> c_int {
        let fd: EvutilSocket = if let Some(fd_) = fd {
            // Actual fd
            fd_
        } else {
            // Timer
            -1
        };

        let callback_ctx = if let Some(_ctx) = callback_ctx {
            _ctx
        } else {
            std::ptr::null::<c_void>() as *mut std::ffi::c_void
        };

        unsafe {
            libevent_sys::event_assign(
                ev.inner.inner,
                self.as_inner_mut(),
                fd,
                flags.bits() as c_short,
                Some(callback),
                callback_ctx,
            )
        }
    }

    pub fn event_add(
        //&mut self,
        & self,
        //event: *mut libevent_sys::event,
        event: &EventHandle,
        timeout: Duration,
    ) -> c_int {
        let tv = to_timeval(timeout);
        unsafe {
            libevent_sys::event_add(event.inner.inner, &tv)
        }
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

bitflags! {
    pub struct EventFlags: u32 {
        const TIMEOUT = libevent_sys::EV_TIMEOUT;
        const READ = libevent_sys::EV_READ;
        const WRITE = libevent_sys::EV_WRITE;
        const SIGNAL = libevent_sys::EV_SIGNAL;
        const PERSIST = libevent_sys::EV_PERSIST;
        const ET = libevent_sys::EV_ET;
        const FINALIZE = libevent_sys::EV_FINALIZE;
        const CLOSED = libevent_sys::EV_CLOSED;
    }
}
