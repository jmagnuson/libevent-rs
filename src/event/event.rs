use std::sync::Arc;

pub struct EventHandle {
    pub inner: Arc<EventHandleInner>,
}

pub struct EventHandleInner {
    pub inner: *mut libevent_sys::event,
}

// This okay, AS LONG AS `EventHandle` isn't Clone...
impl Drop for EventHandleInner {
    fn drop(&mut self) {
        unsafe { libevent_sys::event_free(self.inner) }
    }
}

