use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    sync::futures::Notified,
    task::{JoinError, JoinHandle},
    time::Timeout,
};

/// Runtime interface for dealing with various runtime ownership scenarios
pub trait Runtime {
    fn enter(&self) -> tokio::runtime::EnterGuard<'_>;
    fn join(&self, future: JoinFuture);
    fn dispatch_yield(&self, future: YieldFuture);
    fn dispatch_notify(&self, future: Notified<'_>);
    fn dispatch_timeout(&self, future: Timeout<Notified<'_>>);
}

pub struct JoinFuture {
    join_handle: JoinHandle<()>,
}

impl JoinFuture {
    pub fn new(join_handle: JoinHandle<()>) -> Self {
        Self { join_handle }
    }
}

impl Future for JoinFuture {
    type Output = Result<(), JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let join_handle = Pin::new(&mut self.get_mut().join_handle);

        join_handle.poll(cx)
    }
}

pub struct YieldFuture(u8);

impl Default for YieldFuture {
    fn default() -> Self {
        Self(0)
    }
}

impl Future for YieldFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let counter = self.0;

        if counter < 2 {
            self.get_mut().0 += 1;
            cx.waker().wake_by_ref();

            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub struct TokioRuntime {
    inner: tokio::runtime::Runtime,
}

impl TokioRuntime {
    pub fn new() -> std::io::Result<Self> {
        let inner = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        Ok(Self { inner })
    }
}

impl Runtime for TokioRuntime {
    fn enter(&self) -> tokio::runtime::EnterGuard<'_> {
        self.inner.enter()
    }

    fn join(&self, future: JoinFuture) {
        let _ = self.inner.block_on(future);
    }

    fn dispatch_yield(&self, future: YieldFuture) {
        self.inner.block_on(future);
    }

    fn dispatch_notify(&self, future: Notified<'_>) {
        self.inner.block_on(future);
    }

    fn dispatch_timeout(&self, future: Timeout<Notified<'_>>) {
        let _ = self.inner.block_on(future);
    }
}
