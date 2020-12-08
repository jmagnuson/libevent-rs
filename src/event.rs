use crate::EventFlags;
use std::cell::RefCell;
use std::io;
use std::marker::PhantomData;
use std::os::unix::io::{FromRawFd, RawFd};
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::lock::*;

/// The primitive event-type which is created with [Event::new] using a
/// a non-negative `RawFd`.
///
/// [Event::new]: struct.Event.html#method.new
#[derive(Debug)]
pub struct Fd;

/// A specialized event-type which represents a continuous-interval timer.
#[derive(Debug)]
pub struct Interval;

/// A specialized event-type which represents a one-time event that cleans
/// itself up after execution.
#[derive(Debug)]
pub struct Oneshot;

impl Interval {
    pub fn new(interval: Duration) -> Event<Inactive<Interval>> {
        Inactive::new(None, EventFlags::PERSIST, Some(interval))
    }
}

impl Oneshot {
    pub fn new(timeout: Duration) -> Event<Inactive<Oneshot>> {
        Inactive::new(None, EventFlags::empty(), Some(timeout))
    }
}

/// Internal handle to the raw `event` and other metadata.
#[derive(Debug)]
pub(crate) struct EventInner {
    inner: NonNull<libevent_sys::event>,
    finalizer: libevent_sys::event_finalize_callback_fn,
}

impl EventInner {
    /// Creates a new `EventInner` given a raw `event`, and a "finalizer"
    /// function which helps in `Drop` teardown.
    // TODO: unsafe?
    pub(crate) fn from_raw(
        inner: NonNull<libevent_sys::event>,
        finalizer: libevent_sys::event_finalize_callback_fn,
    ) -> Self {
        EventInner {
            inner,
            finalizer,
        }
    }

    /// Unsafe because other parts rely on `*mut event` being not-null.
    pub(crate) unsafe fn as_raw(&self) -> NonNull<libevent_sys::event> {
        self.inner
    }

    /// Deactivates an event from its corresponding base via `event_del`
    ///
    /// Note that this does not explicitly free the event, but this is handled
    /// automatically, either via the callback wrapper, or RAII. Also, it is
    /// not necessary to call `stop` when dropping the event handle; it will be
    /// handled internally by libevent.
    pub fn stop(&mut self) -> io::Result<()> {
        if (unsafe { libevent_sys::event_del(self.inner.as_ptr()) }) == 0 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Failed to stop event"))
        }
    }

    fn __drop_context(
        event: NonNull<libevent_sys::event>,
        finalizer: libevent_sys::event_finalize_callback_fn,
    ) {
        let ptr = event.as_ptr();
        let ctx = unsafe { libevent_sys::event_get_callback_arg(ptr) };

        unsafe { (finalizer.unwrap())(event.as_ptr(), ctx) };
    }

    /// Uses the stored finalizer function to be able to drop the type-specific
    /// context without the caller actually needing to know those types.
    fn drop_context(&mut self) {
        Self::__drop_context(self.inner, self.finalizer);
    }
}

// Activation & Synchronization variants

/// Inactive event, which defines an event-type `T`.
///
/// Must be activated via one of the `Base::spawn*` function variants.
#[derive(Debug)]
#[doc(hidden)]
pub struct Inactive<T> {
    fd: Option<RawFd>,
    flags: EventFlags,
    timeout: Option<Duration>,
    _phantom: PhantomData<T>,
}

/// Callback-local synchronization type used by `Base::spawn`.
#[derive(Debug)]
#[doc(hidden)]
pub struct Internal<T>(pub(crate) EventInner, PhantomData<T>);

/// Thread-local synchronization type used by `Base::spawn_local`.
#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct Local<T>(pub(crate) Rc<RefCell<EventInner>>, PhantomData<T>);

/// Downgraded version of `Local` which does not count toward ownership.
#[derive(Debug)]
#[doc(hidden)]
pub struct LocalWeak<T>(pub(crate) std::rc::Weak<RefCell<EventInner>>, PhantomData<T>);

/// The exposed event handle which wraps the raw `event` with a defined
/// synchronization method and contains other necessary metadata.
#[derive(Debug)]
#[must_use = "if unused an active event could end by RAII"]
pub struct Event<S> {
    pub(crate) inner: S,
    pub(crate) in_callback: Arc<AtomicBool>,
    pub(crate) stopped: Arc<AtomicBool>,
}

impl<S> Event<S> {
    #![allow(dead_code)]
    #[inline]
    pub(crate) fn in_callback(&self) -> bool {
        self.in_callback.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn set_in_callback(&self, in_cb: bool) {
        self.in_callback.store(in_cb, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn set_stopped(&self, stopped: bool) {
        self.stopped.store(stopped, Ordering::Relaxed);
    }
}

impl Event<Inactive<Fd>> {
    pub fn new(fd: RawFd, flags: EventFlags, timeout: Option<Duration>) -> Self {
        Inactive::new(Some(fd), flags, timeout)
    }

    /// Shouldn't be allowing Fd's to clone, so this is internal-only.
    pub(crate) fn __clone(&self) -> Self {
        Inactive::new(
            self.inactive_fd(),
            self.inactive_flags(),
            self.inactive_timeout(),
        )
    }
}

/// These would normally be part of the `Event` trait, but I want to see if I
/// can develop a more ergonomic API around event types. (i.e., Event<Fd> would
/// expose `pub fn fd()`, but not the timer types.
impl<T> Event<Inactive<T>> {
    pub(crate) fn inactive_fd(&self) -> Option<RawFd> {
        self.inner.fd.as_ref().copied()
    }
    pub(crate) fn inactive_flags(&self) -> EventFlags {
        self.inner.flags
    }
    pub(crate) fn inactive_timeout(&self) -> Option<Duration> {
        self.inner.timeout
    }
}

impl<T> Inactive<T> {
    fn new(fd: Option<RawFd>, flags: EventFlags, timeout: Option<Duration>) -> Event<Self> {
        Event {
            inner: Inactive {
                fd,
                flags,
                timeout,
                _phantom: Default::default(),
            },
            in_callback: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<S: Clone> Clone for Event<S> {
    fn clone(&self) -> Self {
        Event {
            inner: self.inner.clone(),
            in_callback: self.in_callback.clone(),
            stopped: self.stopped.clone(),
        }
    }
}

/// Abstraction over "downgradable" types (i.e., `Rc` and `Arc`).
pub(crate) trait Downgrade {
    type Weak;

    fn downgrade(&self) -> Self::Weak;
}

impl<T> Downgrade for Local<T> {
    type Weak = LocalWeak<T>;

    fn downgrade(&self) -> Self::Weak {
        LocalWeak(Rc::downgrade(&self.0), self.1)
    }
}

impl<S: Downgrade> Downgrade for Event<S> {
    type Weak = Event<S::Weak>;

    fn downgrade(&self) -> Self::Weak {
        Event {
            inner: self.inner.downgrade(),
            in_callback: self.in_callback.clone(),
            stopped: self.stopped.clone(),
        }
    }
}

impl<T> From<EventInner> for Event<Internal<T>> {
    fn from(inner: EventInner) -> Self {
        Event {
            inner: Internal(inner, PhantomData::default()),
            in_callback: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<T> From<EventInner> for Event<Local<T>> {
    fn from(inner: EventInner) -> Self {
        Event {
            inner: Local(Rc::new(RefCell::new(inner)), PhantomData::default()),
            in_callback: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }
}

trait EventMut {
    type EventType;

    fn stop(&mut self) -> io::Result<()>;
}

impl<E: WithInner<In=Self::EventType>> EventMut for E {
    type EventType = EventInner;

    fn stop(&mut self) -> io::Result<()> {
        self.with_inner(|inner| inner.stop())
    }
}


impl<T> Event<Internal<T>> {
    fn with_inner<F, O>(&mut self, f: F) -> O
    where
        F: Fn(&mut EventInner) -> O,
    {
        let ev = &mut self.inner.0;
        f(ev)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        self.set_stopped(true);
        self.with_inner(|inner| inner.stop())
    }
}

impl<T> Event<Local<T>> {
    fn with_inner<F, O>(&self, f: F) -> O
    where
        F: Fn(&mut EventInner) -> O,
    {
        let mut ev = self.inner.0.borrow_mut();
        f(&mut *ev)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        self.set_stopped(true);
        self.with_inner(|inner| inner.stop())
    }
}

impl<T> Event<LocalWeak<T>> {
    fn with_inner<F, O>(&self, f: F) -> O
    where
        F: Fn(&mut EventInner) -> O,
    {
        let upgraded = self.inner.0.upgrade().unwrap();
        let mut ev = upgraded.borrow_mut();
        f(&mut *ev)
    }

    pub fn stop(&mut self) -> io::Result<()> {
        self.set_stopped(true);
        self.with_inner(|inner| inner.stop())
    }
}

impl Drop for EventInner {
    fn drop(&mut self) {
        self.drop_context();

        let raw = unsafe { self.as_raw() };

        unsafe { libevent_sys::event_free(raw.as_ptr()) };
    }
}

/// Provides a typed mapping from libevent's callback signature, to event-type
/// specific closure arguments.
///
/// For example, event-type `Interval` does not have an associated file
/// descriptor, so it is better just to mask the implicitly-invalid fd value
/// libevent passes into the callback.
pub trait Exec<S, F> {
    fn exec(ev: &mut Event<S>, fd: RawFd, flags: EventFlags, cb: &mut F);
}

impl<S, F: FnMut(&mut Event<S>, T, EventFlags), T: FromRawFd> Exec<S, F> for T {
    fn exec(ev: &mut Event<S>, fd: RawFd, flags: EventFlags, cb: &mut F) {
        cb(ev, unsafe { T::from_raw_fd(fd) } , flags)
    }
}

impl<S, F: FnMut(&mut Event<S>, RawFd, EventFlags)> Exec<S, F> for Fd {
    fn exec(ev: &mut Event<S>, fd: RawFd, flags: EventFlags, cb: &mut F) {
        cb(ev, fd, flags)
    }
}

impl<S, F: FnMut(&mut Event<S>)> Exec<S, F> for Interval {
    fn exec(ev: &mut Event<S>, _fd: RawFd, _flags: EventFlags, cb: &mut F) {
        cb(ev)
    }
}

impl<S, F: FnMut(&mut Event<S>)> Exec<S, F> for Oneshot {
    fn exec(ev: &mut Event<S>, _fd: RawFd, _flags: EventFlags, cb: &mut F) {
        cb(ev)
    }
}
