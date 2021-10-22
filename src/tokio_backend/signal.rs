use super::BaseWrapper;
use std::{collections::HashMap, os::raw::c_int, sync::Arc};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::Notify,
    task::JoinHandle,
};

/// Manages adding and removing signal event tasks
#[derive(Debug)]
pub struct SignalMap {
    inner: HashMap<c_int, (Arc<Notify>, JoinHandle<()>)>,
}

impl SignalMap {
    pub(crate) fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn add(
        &mut self,
        base: BaseWrapper,
        signum: c_int,
        dispatch_notify: Arc<Notify>,
    ) -> std::io::Result<()> {
        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();
        let mut stream = signal(SignalKind::from_raw(signum))?;
        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = stream.recv() => {
                        if result.is_some() {
                            unsafe {
                                libevent_sys::evmap_signal_active_(base.0, signum, 1);
                            }
                            dispatch_notify.notify_one();
                        } else {
                            tracing::error!("signal stream has closed");
                            break;
                        }
                    },
                    _ = notify.notified() => {
                        break;
                    }
                }
            }
        });

        self.inner.insert(signum, (notify_clone, join_handle));

        Ok(())
    }

    pub fn del(&mut self, signum: c_int) -> Result<JoinHandle<()>, ()> {
        if let Some((notify, join_handle)) = self.inner.remove(&signum) {
            notify.notify_one();
            Ok(join_handle)
        } else {
            Err(())
        }
    }
}
