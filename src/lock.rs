use std::sync::{Arc, Mutex, Weak as WeakArc};
use std::rc::{Rc, Weak as WeakRc};
use std::cell::RefCell;


pub(crate) trait WithInner {
    type In;

    fn with_inner<F, U: Sized>(&self, f: F) -> U
    where
        F: FnOnce(&mut Self::In) -> U;
}

impl<T> WithInner for Arc<Mutex<T>> {
    type In = T;

    fn with_inner<F, U: Sized>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut t = self.lock().unwrap();
        let u = f(&mut *t);
        u
    }
}

impl<T> WithInner for Rc<RefCell<T>> {
    type In = T;

    fn with_inner<F, U: Sized>(&self, f: F) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut t = self.borrow_mut();
        let u = f(&mut *t);
        u
    }
}

/// Abstraction over "downgradable" types (i.e., `Rc` and `Arc`).
pub(crate) trait Downgrade {
    type Weak;

    fn downgrade(&self) -> Self::Weak;
}

impl<T> Downgrade for Arc<Mutex<T>> {
    type Weak = WeakArc<Mutex<T>>;

    fn downgrade(&self) -> Self::Weak {
        Arc::downgrade(&self)
    }
}

impl<T> Downgrade for Rc<RefCell<T>> {
    type Weak = WeakRc<RefCell<T>>;

    fn downgrade(&self) -> Self::Weak {
        Rc::downgrade(&self)
    }
}

/// Abstraction over "upgradeable" types (i.e., `rc::Weak` and `sync::Weak`).
pub(crate) trait Upgrade {
    type Pointer;

    fn upgrade(&self) -> Option<Self::Pointer>;
}

impl<T> Upgrade for WeakArc<Mutex<T>> {
    type Pointer = Arc<Mutex<T>>;

    fn upgrade(&self) -> Option<Self::Pointer> {
        WeakArc::upgrade(&self)
    }
}

impl<T> Upgrade for WeakRc<RefCell<T>> {
    type Pointer = Rc<RefCell<T>>;

    fn upgrade(&self) -> Option<Self::Pointer> {
        WeakRc::upgrade(&self)
    }
}
