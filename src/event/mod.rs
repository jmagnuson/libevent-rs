mod base;
pub use base::*;

#[allow(clippy::module_inception)]
mod event;
pub use event::*;

use std::ptr::NonNull;
use std::time::Duration;

// Or maybe Deref<Target=NonNull<libevent_sys::event>>?
pub trait AsRawEvent {
    fn as_raw(&mut self) -> NonNull<libevent_sys::event>;
}

/*impl<'a, T> AsRawEvent for &'a T where T: AsRawEvent {
    fn as_raw(&mut self) -> NonNull<libevent_sys::event> {
        (**self).as_raw()
    }
}*/
impl<'a, T> AsRawEvent for &'a mut T where T: AsRawEvent {
    fn as_raw(&mut self) -> NonNull<libevent_sys::event> {
        (**self).as_raw()
    }
}
//impl<'a, T> AsRawEvent

pub trait Loopable {
    fn loop_(&self, flags: LoopFlags) -> ExitReason;
}

pub trait LoopMut {
    fn loopexit(&self, timeout: Duration) -> i32;
    fn loopbreak(&self) -> i32;
    fn loopcontinue(&self) -> i32;
}

pub trait Runnable {
    fn turn(&self) -> ExitReason;
    fn run_timeout(&self, timeout: Duration) -> ExitReason;
    fn run_until_event(&self, timeout: Option<Duration>) -> ExitReason;
    fn run(&self) -> ExitReason;
}

impl <L> Runnable for L
    where
        L: Loopable + LoopMut
{
    fn turn(&self) -> ExitReason {
        self.loop_(LoopFlags::NONBLOCK)
    }

    fn run_timeout(&self, timeout: Duration) -> ExitReason {
        if self.loopexit(timeout) != 0 {
            // TODO: This conflates errors, is it ok?
            return ExitReason::Error;
        };
        self.loop_(LoopFlags::empty())
    }

    fn run_until_event(&self, timeout: Option<Duration>) -> ExitReason {
        if let Some(timeout) = timeout {
            if self.loopexit(timeout) != 0 {
                // TODO: This conflates errors, is it ok?
                return ExitReason::Error;
            }
        }
        self.loop_(LoopFlags::ONCE)
    }

    fn run(&self) -> ExitReason {
        self.loop_(LoopFlags::empty())
    }
}

use std::os::raw::{c_int, c_short, c_void};

pub trait Eventable {
    fn event_assign(
        &mut self,
        mut ev: impl AsRawEvent,
        fd: Option<EvutilSocket>,
        flags: EventFlags,
        callback: EventCallbackFn,
        callback_ctx: Option<EventCallbackCtx>,
    ) -> c_int;

    fn event_add(
        &self,
        event: impl AsRawEvent,
        timeout: Option<Duration>
    ) -> c_int;
}