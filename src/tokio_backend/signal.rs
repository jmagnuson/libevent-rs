use std::{
    collections::HashMap,
    os::raw::c_int,
    task::{Context, Poll},
};
use tokio::signal::unix::{signal, Signal, SignalKind};

#[derive(Debug)]
pub struct SignalMap {
    inner: HashMap<c_int, Signal>,
}

impl SignalMap {
    pub(crate) fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn add(&mut self, signum: c_int) -> std::io::Result<Option<Signal>> {
        let stream = signal(SignalKind::from_raw(signum))?;

        Ok(self.inner.insert(signum, stream))
    }

    pub fn del(&mut self, signum: c_int) -> Option<Signal> {
        self.inner.remove(&signum)
    }

    pub fn dispatch(&mut self, base: *mut libevent_sys::event_base, cx: &mut Context) -> bool {
        let mut ready = false;

        for (signum, stream) in &mut self.inner {
            if let Poll::Ready(Some(())) = stream.poll_recv(cx) {
                unsafe {
                    libevent_sys::evmap_signal_active_(base, *signum, 1);
                }
                ready = true;
            }
        }

        ready
    }
}
