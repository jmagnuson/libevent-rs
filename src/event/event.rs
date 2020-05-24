use std::ptr::NonNull;
use std::os::raw;

pub struct EventHandle {
    pub inner: NonNull<libevent_sys::event>,
    finalizer: Option<Box<dyn FnOnce(&mut Self)>>,
}

impl EventHandle {
    pub(crate) fn new_unchecked(inner: *mut libevent_sys::event) -> Self {
        let inner = NonNull::new(inner).expect("Got null event pointer.");

        EventHandle {
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

unsafe impl Send for EventHandle {}

impl Drop for EventHandle {
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
