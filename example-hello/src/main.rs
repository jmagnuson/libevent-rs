
use std::time::Duration;
use libevent::Libevent;
use libevent_sys;

pub mod ffi;

use std::os::raw::{c_int, c_short, c_void};

type EvutilSocket = c_int;
type EventCallbackFn = extern "C" fn(EvutilSocket, c_short, *mut c_void);

extern "C" fn hello_callback(fd: EvutilSocket, event: c_short, ctx: *mut c_void) {
    println!("Rust callback says hello");
}

fn main() {
    println!("Hello, world!");

    let libevent = Libevent::new()
        .unwrap_or_else(|e| panic!("{:?}", e));

    let _ = unsafe { libevent.with_base(|base| {
        ffi::helloc_init(base)
    })};

    let ev = unsafe { libevent.base().event_new(
        None,
        libevent_sys::EV_PERSIST as c_short,
        hello_callback,
        unsafe {std::mem::transmute(std::ptr::null::<c_void>()) },
    ) };

    let _ = unsafe {
        libevent.base().event_add(ev, Duration::from_secs(2))
    };
    /*let _ev = unsafe { libevent.with_base(|base| {
        base.event_new()
    })};*/

    let mut a: usize = 0;

    let _ = libevent.add_interval(
        Duration::from_secs(6),
        move || {
            a += 1;
            println!("whoaaa {}", a);
        }
    );

    let libevent_ref = &libevent;

    loop {
        let now = std::time::Instant::now();
        libevent_ref.loop_timeout(Duration::from_secs(5));

        let elapsed = now.elapsed();

        println!("Ran libevent loop for {:?}", elapsed);
    }
}
