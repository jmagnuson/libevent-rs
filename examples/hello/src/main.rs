
use std::time::Duration;
use libevent::Libevent;

pub mod ffi;

use libevent::{EvutilSocket, EventCallbackFlags, EventCallbackCtx};

extern "C" fn hello_callback(_fd: EvutilSocket, _event: EventCallbackFlags, _ctx: EventCallbackCtx) {
    println!("Rust callback says hello");
}

fn main() {
    println!("Hello, world!");

    let mut libevent = Libevent::new()
        .unwrap_or_else(|e| panic!("{:?}", e));

    let _ = unsafe { libevent.with_base(|base| {
        ffi::helloc_init(base)
    })};

    let ev = unsafe { libevent.base_mut().event_new(
        None,
        libevent::EventFlags::PERSIST,
        hello_callback,
        None,
    ) };

    let _ = unsafe {
        libevent.base().event_add(&ev, Duration::from_secs(2))
    };

    let mut a: usize = 0;

    let _ev = libevent.add_interval(
        Duration::from_secs(6),
        move |_ev, _flags| {
            a += 1;
            println!("interval count: {}, flags: {:?}", a, _flags);
        }
    );

    for _count in 1..=3 {
        let now = std::time::Instant::now();
        libevent.run_timeout(Duration::from_secs(5));

        let elapsed = now.elapsed();

        println!("Ran libevent loop for {:?}", elapsed);
    }

    libevent.run();
}
