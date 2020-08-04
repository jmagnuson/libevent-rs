use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct EventHandle {
    pub inner: Arc<Mutex<Inner>>,
}

impl EventHandle {
    pub(crate) fn from_raw_unchecked(inner: *mut libevent_sys::event) -> Self {
        EventHandle {
            inner: Arc::new(Mutex::new(Inner::new_unchecked(inner))),
        }
    }
}

pub struct Inner {
    drop_ctx: bool,
    pub inner: Option<NonNull<libevent_sys::event>>,
}

impl Inner {
    pub(crate) fn new_unchecked(inner: *mut libevent_sys::event) -> Self {
        let inner = NonNull::new(inner).expect("Got null event pointer.");

        Inner {
            drop_ctx: false,
            inner: Some(inner),
        }
    }

    // TODO: any better way to do this.
    pub(crate) unsafe fn set_drop_ctx(&mut self) {
        self.drop_ctx = true;
    }

    unsafe fn drop_context(event: NonNull<libevent_sys::event>) {
        let ptr = event.as_ptr();
        let boxed = Box::from_raw((*ptr).ev_evcallback.evcb_arg);
        drop(boxed);
    }
}

unsafe impl Send for Inner {}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            unsafe {
                // Note: This must come before `event_free`.
                if self.drop_ctx {
                    Self::drop_context(inner);
                }

                libevent_sys::event_free(inner.as_ptr());
            }
        }
    }
}
