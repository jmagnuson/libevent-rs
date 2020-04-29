use std::sync::Arc;

pub struct EventHandle {
    pub inner: Arc<EventHandleInner>,
}

pub struct EventHandleInner {
    pub inner: *mut libevent_sys::event,
}

impl EventHandleInner {
    pub(crate) fn _clone(&self) -> Self {
        EventHandleInner {
            inner: self.inner,
        }
    }
}
// This okay, AS LONG AS `EventHandle` isn't Clone...
/*impl Drop for EventHandleInner {
    fn drop(&mut self) {
        unsafe { libevent_sys::event_free(self.inner) }
    }
}*/
