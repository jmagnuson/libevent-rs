use super::*;
use std::ptr::NonNull;
use std::os::raw;

#[cfg(unix)]
use std::os::unix::io::RawFd;
// TODO: #[cfg(windows)] use RawSocket as RawFd;

/// Gets used as the boxed context for `ExternCallbackFn`
struct EventCallbackWrapper {
    inner: Box<dyn FnMut(RawFd, EventFlags)>,
}

extern "C" fn handle_wrapped_callback(fd: EvutilSocket, event: raw::c_short, ctx: EventCallbackCtx) {
    let cb_ref = unsafe {
        let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
        let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
        _cb_ref
    };

    let fd = fd as RawFd;
    let flags = EventFlags::from_bits_truncate(event as u32);
    (cb_ref.inner)(fd, flags)
}

pub struct FdEvent {
    pub inner: NonNull<libevent_sys::event>,
    finalizer: Option<Box<dyn FnOnce(&mut Self)>>,
}

impl FdEvent {
    pub(crate) fn new_unchecked(inner: *mut libevent_sys::event) -> Self {
        let inner = NonNull::new(inner).expect("Got null event pointer.");

        FdEvent {
            inner,
            finalizer: None,
        }
    }

    pub(crate) fn set_finalizer<F>(&mut self, finalizer: F)
        where
            F: FnOnce(&mut Self) + 'static
    {
        self.finalizer = Some(Box::new(finalizer));
    }

    pub fn base(&self) -> base::Handle {

    }
}

unsafe impl Send for FdEvent {}

impl Drop for FdEvent {
    fn drop(&mut self) {
        if let Some(finalizer) = self.finalizer.take() {
            (finalizer)(self);
        }

        unsafe {
            println!("FREEING EVENT POINTER");
            libevent_sys::event_free(self.inner.as_ptr());
        }
    }
}

impl AsRawEvent for FdEvent {
    fn as_raw(&mut self) -> NonNull<libevent_sys::event> {
        self.inner
    }
}

use super::base::EventBase;
impl Event for FdEvent {
    #[cfg(unix)]
    fn fd(&self) -> std::os::unix::io::RawFd {
        libevent_sys::event_get_fd(self.as_raw().as_ptr()) as RawFd
    }

    // TODO: with_base? can't return a ref here
    fn base(&self) -> &EventBase {
        unimplemented!()
        //EventBase::from_raw((*self.as_raw().as_ptr()).ev_base)
    }

    fn flags(&self) -> super::base::EventFlags {
        EventFlags::from_bits_truncate(
            libevent_sys::event_get_events(self.as_raw().as_ptr()) as u32
        )
    }

    fn cb(&self) -> libevent_sys::event_callback_fn {
        //(*self.as_raw().as_ptr()).ev_evcallback
        libevent_sys::event_get_callback(self.as_raw().as_ptr())
    }

    fn cb_arg(&self) -> *mut raw::c_void {
        //(*self.as_raw().as_ptr()).ev_fd as RawFd
        libevent_sys::event_get_callback_arg(self.as_raw().as_ptr())
    }

    fn priority(&self) -> raw::c_int {
        libevent_sys::event_get_priority(self.as_raw().as_ptr())
    }

    fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(&mut Self)>) {
        (*self.as_raw().as_ptr()).ev_fd as RawFd
    }

    //fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(NonNull<libevent_sys::event>)>);
    fn timeout(&self) -> Option<std::time::Duration>{
        // Hmm.. this may be a Instant instead
        super::base::from_timeval((*self.as_raw().as_ptr()).ev_timeout)
    }
}

pub/*(crate)*/ trait Event: AsRawEvent {
    #[cfg(unix)]
    fn fd(&self) -> std::os::unix::io::RawFd;
    fn base(&self) -> &super::base::EventBase;
    fn flags(&self) -> super::base::EventFlags;
    fn cb(&self) -> libevent_sys::event_callback_fn;
    fn cb_arg(&self) -> *mut raw::c_void;
    fn priority(&self) -> raw::c_int;
    fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(&mut Self)>);
    //fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(NonNull<libevent_sys::event>)>);
    fn timeout(&self) -> Option<std::time::Duration>;
    fn struct_size() -> libevent_sys::size_t {
        unsafe { libevent_sys::event_get_struct_event_size() }
    }
}

pub trait EventMut {
    fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(&mut Self)>);
    /// Stops event, but does not deallocate it. Used in event callback.
    fn stop(&mut self);
}

//impl<'a, T> Event for &'a T where T: Event {}
//impl<'a, T> Event for &'a mut T where T: Event + AsRawEvent {}

/*
impl<'a, T> Event for &'a mut T where T: Event {
    #[cfg(unix)]
    fn fd(&self) -> std::os::unix::io::RawFd { (**self).fd() }
    fn base(&self) -> &super::base::EventBase { (**self).base() }
    fn flags(&self) -> super::base::EventFlags { (**self).flags() }
    fn cb(&self) -> libevent_sys::event_callback_fn { (**self).cb() }
    fn cb_arg(&self) -> *mut raw::c_void { (**self).cb_arg() }
    fn priority(&self) -> raw::c_int { (**self).priority() }
    fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(Self)>) { (**self).set_finalizer(finalizer) }
    //fn set_finalizer(&mut self, finalizer: Box<dyn FnOnce(NonNull<libevent_sys::event>)>);
    fn timeout(&self) -> Option<std::time::Duration> { (**self).timeout() }
}
*/
