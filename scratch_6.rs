
use std::io;
use std::rc::{self, Rc};
use std::time::Duration;

struct Base;
enum Flags {
    Foo,
    Bar,
}

trait Event {

}

impl Base {
    pub fn spawn<F, T>(&mut self, f: F, ev: T) -> io::Result<()>
    where
        F: FnMut(RawFd, Flags, &mut T) -> Option<()>,
        T: Event,
    {
        let mut ev = ev;
        ev.set_finalizer();
        self.base_mut().event_assign(&mut ev, );
    }

    pub fn spawn_shared<F, T>(&mut self, f: F, ev: T) -> io::Result<Rc<T>>
        where
            F: FnMut(RawFd, Flags, rc::Weak<T>) -> Option<()>,
            T: Event,
    {
        let shared_ev = Rc::new(RefCell::new(ev));
        ev.set_finalizer();
        self.base_mut().event_assign(ev, );
    }
}

struct FdEvent;
mod timer {
    pub struct Interval;
    pub struct Oneshot;
}

impl timer::Interval {
    pub fn new(base: Base, interval: Duration) -> Self {

    }
}

struct EventBuilder {
    ev: NonNull<libevent_sys::event>,
    fd: Option<RawFd>,
}

impl EventBuilder {
    pub fn timer(self) -> Self {
        self.fd = None;
    }
}

fn arc_spawn() {
    let event = Arc::new(RefCell::new(base.event_new(params, other, than, closure)));

    // Will get moved into the closure.
    let closure_event = event.weak();
    {
        // unlock long enough to spawn on base.
        let unlocked_event = event.lock().unwrap();
        unlocked_event.spawn(move |fd, flags| {
            if fd.read().is_err() {
                closure_event.lock().unwrap().do_something();
            }
        });
        // ..OR..
        base.spawn(unlocked_event, move |fd, flags| {
            if fd.read().is_err() {
                closure_event.lock().unwrap().do_something();
            }
        })
    }
}

fn event_spawn_shared() {
    struct Base;
    impl Base {
        pub fn spawn<F, T>(&mut self, ev: T, f: F) -> io::Result<()>
        where
            F: FnMut(RawFd, Flags, &mut T) -> Option<()>,
            T: Event,
        {
            let mut ev = ev;
            ev.set_finalizer();
            self.base_mut().event_assign(&mut ev, );

            Ok(())
        }

        pub fn spawn_shared<F, T>(&mut self, ev: T, f: F) -> io::Result<Rc<RefCell<T>>>
        where
            F: FnMut(RawFd, Flags, rc::Weak<RefCell<T>>) -> Option<()>,
            T: Event,
        {
            let mut ev = ev;
            ev.set_finalizer();
            self.base_mut().event_assign(&mut ev, );

            Rc::new(RefCell::new(ev))
        }

        pub fn spawn_sync<F, T>(&mut self, ev: T, f: F) -> io::Result<Arc<Mutex<T>>>
        where
            F: FnMut(RawFd, Flags, sync::Weak<Mutex<T>>) -> Option<()>,
            T: Event,
        {
            let mut ev = ev;
            ev.set_finalizer();
            self.base_mut().event_assign(&mut ev, );

            Arc::new(Mutex::new(ev))
        }

        // .. AND/OR ..

        pub fn add_fd_sync<F>(&mut self, ev: RawFd, other, params, f: F) -> io::Result<Arc<Mutex<Event>>>
        where
            F: FnMut(RawFd, Flags, sync::Weak<Mutex<FdEvent>>) -> Option<()>,
        {
            let mut ev = ev;
            ev.set_finalizer();
            self.base_mut().event_assign(&mut ev, );

            Arc::new(Mutex::new(ev))
        }
    }

    let event = base.event_new(params, other, than, closure);
    let shared_event = base.spawn_shared(event, move |fd, flags, closure_event| {
        if fd.read().is_err() {
            closure_event.lock().unwrap().do_something();
        }
    });
    // ..OR..
    let shared_event = base.add_fd_sync(fd, other, params, move |fd, flags, closure_event| {
        if fd.read().is_err() {
            closure_event.lock().unwrap().do_something();
        }
    });
}

mod extra_flags {
    use std::io;

    struct Event {
        stopped: bool,
        // in_callback: AtomicBool,
    }

    impl Event {
        pub fn stop(&mut self) -> io::Result<()> {
            self.stopped = true;

            // :magic: get ctx pointer from raw event :magic:

            let in_callback = {
                let cb_ref = unsafe {
                    let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
                    let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
                    _cb_ref
                };

                cb_ref.stopped.store(true, Ordering::Relaxed);
                cb_ref.in_callback.load(Ordering::Relaxed)
            };

            if !in_callback {
                // destroyyy
            }

        }
    }

    impl Drop for Event {
        fn drop(&mut self) {
            // :magic: get ctx pointer from raw event :magic:

            let in_callback = {
                let cb_ref = unsafe {
                    let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
                    let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
                    _cb_ref
                };

                cb_ref.in_callback.load(Ordering::Relaxed)
            };

            if !in_callback {
                // destroyyy
            }
        }
    }

    /// Gets used as the boxed context for `ExternCallbackFn`
    struct EventCallbackWrapper {
        inner: Box<dyn FnMut(RawFd, EventFlags, Event,)>,
        event: Event,
        in_callback: AtomicBool,
        stopped: AtomicBool,
    }

    extern "C" fn handle_wrapped_callback(
        fd: EvutilSocket,
        event: raw::c_short,
        ctx: EventCallbackCtx
    ) {
        let cb_ref = unsafe {
            let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
            let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
            _cb_ref
        };

        let fd = fd as RawFd;
        let flags = EventFlags::from_bits_truncate(event as u32);

        cb_ref.in_callback.store(true, Ordering::Relaxed);
        (cb_ref.inner)(fd, flags);
        cb_ref.in_callback.store(false, Ordering::Relaxed);
    }

    /// Gets used as the boxed context for `ExternCallbackFn`
    struct EventCallbackWrapperSync {
        inner: Box<dyn FnMut(RawFd, EventFlags, Event,)>,
        event: Arc<Mutex<Event>>, // TODO: Do I need Mutex?? ref dbus-rs
        in_callback: AtomicBool,
        stopped: AtomicBool, // set by Event::stop
    }
    extern "C" fn handle_wrapped_callback_sync(
        fd: EvutilSocket,
        event: raw::c_short,
        ctx: EventCallbackCtx
    ) {
        let cb_ref = unsafe {
            let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
            let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
            _cb_ref
        };

        let fd = fd as RawFd;
        let flags = EventFlags::from_bits_truncate(event as u32);

        cb_ref.in_callback.store(true, Ordering::Relaxed);
        (cb_ref.inner)(fd, flags);
        cb_ref.in_callback.store(false, Ordering::Relaxed);

    }

}

mod struct_type_impl_macros {
    //! Sets up CallbackFn, Finalizer, etc. for each type (too messy? not sure
    //! how else to do it)

    struct Interval {
        inner: (),
        interval: Duration,
    }

    trait HandleWrappedCallback {
        fn cb() -> extern "C" fn(fd: EvutilSocket, event: c_short, ctx: EventCallbackCtx);
    }

    // bench_ahrs!(_bench_madgwick_update,           Madgwick, update,     1);
    macro_rules! impl_event_struct {
        ($t: ident) => {
            impl HandleWrappedCallback for $t {
                fn cb() -> extern "C" fn(fd: EvutilSocket, event: c_short, ctx: EventCallbackCtx) {
                    extern "C" fn handle_wrapped_callback(fd: EvutilSocket, event: c_short, ctx: EventCallbackCtx) {
                        let cb_ref = unsafe {
                            let cb: *mut EventCallbackWrapper = ctx as *mut EventCallbackWrapper;
                            let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
                            _cb_ref
                        };

                        let flags = EventFlags::from_bits_truncate(event as u32);
                        //let event_handle = &mut cb_ref.ev;
                        (cb_ref.inner)(fd as RawFd, flags)
                    }
                    handle_wrapped_callback
                }
            }

        };
    }

    impl_event_struct!(Interval);

    impl Base {
        pub fn add_event<F, E>(&mut self, mut ev: E, cb: F) -> io::Result<()>
            where
                F: FnMut(RawFd, EventFlags) + Send + 'static,
                E: Event + HandleWrappedCallback,
        {

        }
    }

    // or I could just punt on trying to abstract across everyting and focus on
    // the Event + Sync stuff...
}
