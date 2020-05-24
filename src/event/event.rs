use std::ptr::NonNull;
use std::sync::{Arc, Mutex, Weak};
use std::os::raw;

#[derive(Clone)]
pub struct EventHandle {
    pub inner: Arc<Mutex<Inner>>,
}

/// Used within closures, doesn't count toward ownership.
#[derive(Clone)]
pub struct EventWeakHandle {
    pub inner: Weak<Mutex<Inner>>,
}


impl EventHandle {
    pub(crate) fn from_raw_unchecked(inner: *mut libevent_sys::event) -> Self {
        EventHandle {
            inner: Arc::new(Mutex::new(Inner::new_unchecked(inner))),
        }
    }

    pub(crate) fn weak_handle(&self) -> EventWeakHandle {
        EventWeakHandle {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

pub struct Inner {
    pub inner: Option<NonNull<libevent_sys::event>>,
    finalizer: Option<Box<dyn FnOnce(&mut Inner)>>,
}

impl Inner {
    pub(crate) fn new_unchecked(inner: *mut libevent_sys::event) -> Self {
        let inner = NonNull::new(inner).expect("Got null event pointer.");

        Inner {
            inner: Some(inner),
            finalizer: None,
        }
    }

    pub(crate) fn set_finalizer<F>(&mut self, finalizer: F)
        where
            F: FnOnce(&mut Inner) + 'static
    {
        self.finalizer = Some(Box::new(finalizer));
    }
}

unsafe impl Send for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(finalizer) = self.finalizer.take() {
            (finalizer)(self);
        }

        if let Some(inner) = self.inner.take() {
            unsafe {
                libevent_sys::event_free(inner.as_ptr());
            }
        }
    }
}

pub(crate) trait Event {
    #[cfg(unix)]
    fn fd(&self) -> std::os::unix::io::RawFd;
    fn base(&self) -> &super::base::EventBase;
    fn flags(&self) -> super::base::EventFlags;
    fn cb(&self) -> libevent_sys::event_callback_fn;
    fn cb_arg(&self) -> *mut raw::c_void;
    fn priority(&self) -> raw::c_int;
    fn struct_size() -> libevent_sys::size_t {
        unsafe { libevent_sys::event_get_struct_event_size() }
    }
}
