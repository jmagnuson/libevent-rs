use std::ptr::NonNull;
use std::os::raw;

/// Gets used as the boxed context for `ExternCallbackFn`
struct EventCallbackWrapper {
    inner: Box<dyn FnMut(EventFlags)>,
}

extern "C" fn handle_wrapped_callback(_fd: EvutilSocket, event: c_short, ctx: EventCallbackCtx) {
    let cb_ref = unsafe {
        let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
        let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
        _cb_ref
    };

    let flags = EventFlags::from_bits_truncate(event as u32);
    (cb_ref.inner)(flags)
}

pub struct Interval {
    inner: NonNull<libevent_sys::event>,
    finalizer: Option<Box<dyn FnOnce(&mut Self)>>,
}

impl Interval {
    pub(crate) fn new_unchecked(inner: *mut libevent_sys::event) -> Self {
        let inner = NonNull::new(inner).expect("Got null event pointer.");

        Interval {
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
}

pub struct Oneshot {
    inner: Option<NonNull<libevent_sys::event>>,
    finalizer: Option<Box<dyn FnOnce(&mut Self)>>,
}

impl Oneshot {
    pub(crate) fn new_unchecked(inner: *mut libevent_sys::event) -> Self {
        let inner = NonNull::new(inner).expect("Got null event pointer.");

        Oneshot {
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
}
