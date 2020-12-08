//#![feature(generic_associated_types)]

//use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;


trait LockFamily {
    type Lock<T>: WithInner/*<T>*/;
    fn new<T>(value: T) -> Self::Lock<T>;
}

trait WithInner/*<T>*/ {
    //type InnerFun<U>: FnOnce(&mut T) -> U;
    //type Out;
    type Slef<'a>;
    type In;
    //type Out;

    fn with_inner<F, U: Sized>(Self::Slef, f: F) -> U /*Self::Out*/
    where
        F: FnOnce(&mut /*T*/ Self::In) -> /*Self::Out*/ U;
}

impl<T> WithInner/*<T>*/ for Arc<Mutex<T>> {
    //type InnerFun<U>: FnOnce(&mut T) -> U;
    type Slef<'a> = &'a Arc<Mutex<T>>;
    type In = T;

    fn with_inner<F, U: Sized>(&self, f: F /*Self::InnerFun<U>*/) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut t = self.lock().unwrap();
        let u = f(&mut *t);
        u
    }
}

struct ArcMutexFamily;

impl LockFamily for ArcMutexFamily {
    type Lock<T> = Arc<Mutex<T>>;
    fn new<T>(value: T) -> Self::Lock<T> {
        Arc::new(Mutex::new(value))
    }
}

struct RcRefCellFamily;

impl LockFamily for RcRefCellFamily {
    type Lock<T> = Rc<RefCell<T>>;
    fn new<T>(value: T) -> Self::Lock<T> {
        Rc::new(RefCell::new(value))
    }
}

impl<T> WithInner/*<T>*/ for Rc<RefCell<T>> {
    //type InnerFun<U>: FnOnce(&mut T) -> U;
    type In = T;

    fn with_inner<F, U: Sized>(&self, f: F /*Self::InnerFun<U>*/) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let mut t = self.borrow_mut();
        let u = f(&mut *t);
        u
    }
}

struct NoLockFamily;
struct NoLock<T>(T);

impl LockFamily for NoLockFamily {
    type Lock<T> = NoLock<T>;
    fn new<T>(value: T) -> Self::Lock<T> {
        NoLock(value)
    }
}

impl<T> WithInner/*<T>*/ for NoLock<T> {
    //type InnerFun<U>: FnOnce(&mut T) -> U;
    type In = T;

    fn with_inner<F, U: Sized>(&mut self, f: F /*Self::InnerFun<U>*/) -> U
    where
        F: FnOnce(&mut T) -> U,
    {
        let t = &mut self.0;
        let u = f(t);
        u
    }
}


/*struct Foo<P: LockFamily> {
    bar: P::Pointer<String>,
}*/
