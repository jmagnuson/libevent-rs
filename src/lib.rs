//! Rust bindings to the [libevent] async I/O framework.
//!
//! [libevent]: https://libevent.org/


#![feature(generic_associated_types)]

use std::time::Duration;

mod event;
pub use event::{Event, Fd, Interval, Oneshot};

mod base;
pub use base::{
    Base, EventCallbackCtx, EventCallbackFlags, EventFlags, EvutilSocket, ExitReason, LoopFlags,
};

mod lock;

/// The context passed into `handle_wrapped_callback`, which handles event-type
/// specific metadata for trampolining into the user-supplied closure.
pub(crate) struct EventCallbackWrapper<S, T, F> {
    inner: F,
    event: Option<Event<S>>,
    _phantom: std::marker::PhantomData<T>,
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
}
