mod api;
mod backend;
mod io;
mod runtime;
mod signal;

pub use api::inject_tokio;
pub use runtime::{JoinFuture, Runtime, TokioRuntime, YieldFuture};

/// Wrapper to allow sending of raw event_base pointers to tokio tasks.
///
/// This is safe because libevent performs locking internally.
#[derive(Debug)]
pub(crate) struct BaseWrapper(*mut libevent_sys::event_base);

unsafe impl Send for BaseWrapper {}
