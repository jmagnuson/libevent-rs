#![allow(dead_code)]

use bitflags::bitflags;
use std::io;
use std::os::raw::{c_int, c_short, c_void};
use std::ptr::NonNull;
use std::time::Duration;

use super::event::*;
use crate::lock::Downgrade;
use crate::EventCallbackWrapper;

/// A file descriptor in libevent.
pub type EvutilSocket = c_int;

/// The event callback function in libevent.
pub type EventCallbackFn = extern "C" fn(EvutilSocket, EventCallbackFlags, EventCallbackCtx);

/// The event callback's raw context type (void pointer).
pub type EventCallbackCtx = *mut c_void;

/// The event callback's raw flags type.
pub type EventCallbackFlags = c_short;

/// Convenience function for mapping Rust's `Duration` to libevent's `timeval`.
fn to_timeval(duration: Duration) -> libevent_sys::timeval {
    libevent_sys::timeval {
        tv_sec: duration.as_secs() as _,
        tv_usec: duration.subsec_micros() as _,
    }
}

/// Wrapper for libevent's `event_base` which is responsible for executing
/// associated events.
pub struct Base {
    base: NonNull<libevent_sys::event_base>,
}

/// The handle that abstracts over libevent's API in Rust.
impl Base {
    /// Creates a new instance of `Base`.
    pub fn new() -> Result<Self, io::Error> {
        let base = unsafe { libevent_sys::event_base_new() };

        if let Some(base) = NonNull::new(base) {
            Ok(unsafe { Self::from_raw(base) })
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to create libevent base",
            ))
        }
    }

    /// Creates a new instance of `Base` using a raw, non-null `event_base`
    /// pointer.
    ///
    /// # Safety
    ///
    /// This function expects a non-null pointer, and thus does no such checks
    /// internally. Thus the caller is responsible for checking the
    /// `event_base` validity.
    pub unsafe fn from_raw(base: NonNull<libevent_sys::event_base>) -> Self {
        Base { base }
    }

    /// Exposes the raw, non-null `event_base` pointer.
    ///
    /// # Safety
    ///
    /// This function returns a valid, non-null `event_base` pointer which by
    /// itself is safe. However, this function serves as an escape hatch to do
    /// unsafe things.
    pub unsafe fn as_raw(&self) -> NonNull<libevent_sys::event_base> {
        self.base
    }

    /// Wrapper for libevent's `event_base_loop`, which runs the event loop in
    /// a manner defined by the `LoopFlags` input.
    pub fn loop_(&self, flags: LoopFlags) -> ExitReason {
        let exit_code = unsafe {
            libevent_sys::event_base_loop(self.base.as_ptr(), flags.bits() as i32) as i32
        };

        match exit_code {
            0 => {
                unsafe {
                    // Technically mutually-exclusive from `got_break`, but
                    // the check in `event_base_loop` comes first, so the logic
                    // here matches.
                    if libevent_sys::event_base_got_exit(self.base.as_ptr()) != 0i32 {
                        ExitReason::GotExit
                    } else if libevent_sys::event_base_got_break(self.base.as_ptr()) != 0i32 {
                        ExitReason::GotBreak
                    } else {
                        // TODO: This should match flags for `EVLOOP_ONCE`, `_NONBLOCK`, etc.
                        ExitReason::Unknown { flags, exit_code }
                    }
                }
            }
            -1 => ExitReason::Error,
            1 => ExitReason::NoPendingEvents,
            _ => ExitReason::Unknown { flags, exit_code },
        }
    }

    /// Wrapper for libevent's `event_base_loopexit`, which tells the running
    /// event loop to exit after a specified `Duration`.
    pub fn loopexit(&self, timeout: Duration) -> i32 {
        let tv = to_timeval(timeout);
        unsafe {
            let tv_cast = &tv as *const libevent_sys::timeval;
            libevent_sys::event_base_loopexit(self.as_raw().as_ptr(), tv_cast) as i32
        }
    }

    /// Wrapper for libevent's `event_base_loopbreak`, which tells the running
    /// event loop to break immediately.
    pub fn loopbreak(&self) -> i32 {
        unsafe { libevent_sys::event_base_loopbreak(self.as_raw().as_ptr()) as i32 }
    }

    /// Wrapper for libevent's `event_base_loopcontinue`, which tells the
    /// running event loop to resume searching for active events.
    pub fn loopcontinue(&self) -> i32 {
        unsafe { libevent_sys::event_base_loopcontinue(self.as_raw().as_ptr()) as i32 }
    }

    /// Wrapper for libevent's `event_new`, which allocates and initializes a
    /// new `event` with the given parameters.
    pub fn event_new(
        &mut self,
        fd: Option<EvutilSocket>,
        flags: EventFlags,
        callback: EventCallbackFn,
        callback_ctx: Option<EventCallbackCtx>,
    ) -> Option<NonNull<libevent_sys::event>> {
        let fd: EvutilSocket = if let Some(fd) = fd {
            // Actual fd
            fd
        } else {
            // Timer
            -1
        };

        let callback_ctx = if let Some(ctx) = callback_ctx {
            ctx
        } else {
            std::ptr::null::<c_void>() as *mut std::ffi::c_void
        };

        let inner = unsafe {
            libevent_sys::event_new(
                self.as_raw().as_ptr(),
                fd,
                flags.bits() as c_short,
                Some(callback),
                callback_ctx,
            )
        };

        NonNull::new(inner)
    }

    /// Wrapper for libevent's `event_new`, which initializes a pre-allocated
    /// `event` with the given parameters.
    pub fn event_assign(
        &mut self,
        ev: NonNull<libevent_sys::event>,
        fd: Option<EvutilSocket>,
        flags: EventFlags,
        callback: EventCallbackFn,
        callback_ctx: Option<EventCallbackCtx>,
    ) -> c_int {
        let fd: EvutilSocket = if let Some(fd) = fd {
            // Actual fd
            fd
        } else {
            // Timer
            -1
        };

        let callback_ctx = if let Some(ctx) = callback_ctx {
            ctx
        } else {
            std::ptr::null::<c_void>() as *mut std::ffi::c_void
        };

        unsafe {
            libevent_sys::event_assign(
                ev.as_ptr(),
                self.as_raw().as_ptr(),
                fd,
                flags.bits() as c_short,
                Some(callback),
                callback_ctx,
            )
        }
    }

    /// Wrapper for libevent's `event_add`, which activates an initialized
    /// `event` for a pre-defined `Base` and a given timeout interval.
    pub fn event_add(
        &self,
        event: NonNull<libevent_sys::event>,
        timeout: Option<Duration>,
    ) -> c_int {
        unsafe {
            let p = event.as_ptr();
            if let Some(tv) = timeout {
                libevent_sys::event_add(p, &to_timeval(tv))
            } else {
                // null timeout means no timeout to libevent
                libevent_sys::event_add(p, std::ptr::null())
            }
        }
    }
}

unsafe impl Send for Base {}

impl<S, T: Exec<S, F>, F> EventCallbackWrapper<S, T, F> {
    pub fn new(inner: F, event: Event<S>) -> Box<Self> {
        Box::new(Self {
            inner,
            event: Some(event),
            _phantom: std::marker::PhantomData::default(),
        })
    }
}

/// Handles freeing the callback wrapper memory.
///
/// It is set up in a fashion similar to `handle_wrapped_callback` so that
/// callers do not need to know the type parameters, only that they have a
/// function pointer to a monomorphized function that is valid for a given
/// event context.
pub(crate) unsafe extern "C" fn finalize_wrapped_callback<S, T, F>(
    event: *mut libevent_sys::event,
    ctx: EventCallbackCtx,
) {
    // Wrapper was allocated with Box, now free it with Drop.
    let cb: *mut EventCallbackWrapper<S, T, F> = ctx as *mut EventCallbackWrapper<S, T, F>;
    let owned_cb = Box::from_raw(cb);
    drop(owned_cb);

    // Now clear the event's ctx pointer field.
    let null_ctx = std::ptr::null::<c_void>() as *mut std::ffi::c_void;
    (*event).ev_evcallback.evcb_arg = null_ctx;
}

/// Acts as a C-compatible trampoline for the user-defined callback closure.
pub(crate) extern "C" fn handle_wrapped_callback<S, T, F>(
    fd: EvutilSocket,
    event: std::os::raw::c_short,
    ctx: EventCallbackCtx,
) where
    T: Exec<S, F>,
{
    let cb_ref = unsafe {
        let cb: *mut EventCallbackWrapper<S, T, F> = ctx as *mut EventCallbackWrapper<S, T, F>;
        let _cb_ref: &mut EventCallbackWrapper<S, T, F> = &mut *cb;
        _cb_ref
    };

    let flags = EventFlags::from_bits_truncate(event as u32);
    let ev = cb_ref.event.as_mut().expect("Missing event for callback");

    ev.set_in_callback(true);
    <T as Exec<S, F>>::exec(ev, fd, flags, &mut cb_ref.inner);
    ev.set_in_callback(false);

    // row, row, row your boat..
    if ev.stopped() {
        let event = cb_ref.event.take().expect("Missing event for drop");
        drop(event)
    }
}

impl Base {
    /// Helper for creating a raw event from Inactive.
    fn new_event_raw<S, T: Exec<S, F>, F>(
        &mut self,
        ev: &Event<Inactive<T>>,
    ) -> Option<NonNull<libevent_sys::event>> {
        self.event_new(
            ev.inactive_fd(),
            ev.inactive_flags(),
            handle_wrapped_callback::<S, T, F>,
            None,
        )
    }

    /// Helper for spawning with all necessary components.
    fn assign_event_raw<S, T: Exec<S, F>, F>(
        &mut self,
        ev: &Event<Inactive<T>>,
        raw_ev: NonNull<libevent_sys::event>,
        cb_wrapped: Box<EventCallbackWrapper<S, T, F>>,
    ) -> c_int {
        // Leak the callback wrapper so we can store it as ctx.
        let ctx_ptr = NonNull::from(Box::leak(cb_wrapped));

        self.event_assign(
            raw_ev,
            ev.inactive_fd(),
            ev.inactive_flags(),
            handle_wrapped_callback::<S, T, F>,
            Some(ctx_ptr.as_ptr() as EventCallbackCtx),
        )
    }

    /// Activates a given inactive `Event` with no handle sharing.
    ///
    /// Control of the event via the `Event` handle is relegated only from
    /// within the closure `F`, which means that no synchronization wrappers
    /// are required for operation.
    pub fn spawn<T: Exec<Internal<T>, F>, F>(
        &mut self,
        ev: Event<Inactive<T>>,
        cb: F,
    ) -> io::Result<()> {
        // First allocate the event with no context, then apply the reference
        // to the closure (and itself) later on.
        let raw_ev = self
            .new_event_raw::<Internal<T>, T, F>(&ev)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to allocate event"))?;

        let event: Event<Internal<T>> =
            EventInner::from_raw(raw_ev, Some(finalize_wrapped_callback::<Internal<T>, T, F>))
                .into();

        let cb_wrapped = EventCallbackWrapper::new(cb, event);

        // Now we can apply the closure + handle to self.
        if self.assign_event_raw(&ev, raw_ev, cb_wrapped) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to assign event",
            ));
        }

        if self.event_add(raw_ev, ev.inactive_timeout()) != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to add event"));
        }

        Ok(())
    }

    /// Activates a given inactive `Event` with thread-local sharing.
    ///
    /// Control of the event via the `Event` handle is shared between the
    /// closure `F` as well as the returned `Event`, which internally uses an
    /// `Rc<RefCell>`.
    pub fn spawn_local<T: Exec<LocalWeak<T>, F>, F>(
        &mut self,
        ev: Event<Inactive<T>>,
        cb: F,
    ) -> io::Result<Event<Local<T>>> {
        // First allocate the event with no context, then apply the reference
        // to the closure (and itself) later on.
        let raw_ev = self
            .new_event_raw::<LocalWeak<T>, T, F>(&ev)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to allocate event"))?;

        let event: Event<Local<T>> = EventInner::from_raw(
            raw_ev,
            Some(finalize_wrapped_callback::<LocalWeak<T>, T, F>),
        )
        .into();
        let closure_event = event.downgrade();

        let cb_wrapped = EventCallbackWrapper::new(cb, closure_event);

        // Now we can apply the closure + handle to self.
        if self.assign_event_raw(&ev, raw_ev, cb_wrapped) != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to assign event",
            ));
        }

        if self.event_add(raw_ev, ev.inactive_timeout()) != 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to add event"));
        }

        Ok(event)
    }
}

/// Enumerates all possible reasons that the event loop may have stopped
/// running.
pub enum ExitReason {
    GotExit,
    GotBreak,
    Error,
    NoPendingEvents,
    Unknown { flags: LoopFlags, exit_code: i32 },
}

bitflags! {
    /// Flags given to the event loop to alter its behavior.
    pub struct LoopFlags: u32 {
        const ONCE = libevent_sys::EVLOOP_ONCE;
        const NONBLOCK = libevent_sys::EVLOOP_NONBLOCK;
        const NO_EXIT_ON_EMPTY = libevent_sys::EVLOOP_NO_EXIT_ON_EMPTY;
    }
}

bitflags! {
    /// Flags used both as inputs to define activation characteristics of an event,
    /// as well as an output given in the callback as to what triggered event
    /// activation.
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
