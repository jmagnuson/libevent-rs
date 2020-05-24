use libevent::Libevent;
use std::time::Duration;

pub mod ffi;

use libevent::{EventCallbackCtx, EventCallbackFlags, EvutilSocket};

extern "C" fn hello_callback(
    _fd: EvutilSocket,
    _event: EventCallbackFlags,
    _ctx: EventCallbackCtx,
) {
    println!("callback: rust fn (interval: 2s)");
}

fn main() {
    let run_duration = std::env::args().nth(1).map(|val_s| {
        Duration::from_secs(
            val_s
                .parse()
                .expect("Bad run duration argument (should be in seconds)"),
        )
    });

    unsafe {
        //libevent_sys::event_enable_debug_mode();
        //libevent_sys::event_enable_debug_logging(libevent_sys::EVENT_DBG_ALL);
    }

    let mut libevent = Libevent::new().unwrap_or_else(|e| panic!("{:?}", e));

    let ret = unsafe { libevent.with_base(|base| ffi::helloc_init(base)) };
    assert_eq!(ret, 0);

    let ev = unsafe {
        libevent
            .base_mut()
            .event_new(None, libevent::EventFlags::PERSIST, hello_callback, None)
    };

    let _ = unsafe { libevent.base().event_add(&ev, Some(Duration::from_secs(2))) };

    let mut a: usize = 0;
    let mut b: usize = 0;

    let mut ev_temp = Some(libevent.add_interval(Duration::from_secs(1), move |_ev, _flags| {
        b += 1;
        println!("callback: rust temp closure (interval: 1s, count: {})", b);
    }).unwrap());

    let _ev = libevent.add_interval(Duration::from_secs(6), move |_ev, _flags| {
        a += 1;
        println!("callback: rust closure (interval: 6s, count: {}, flags: {:?})", a, _flags);

        if a == 2 {
            if let Some(inner) = ev_temp.take() {
                println!("Dropping ev_temp timer");
                drop(inner)
            }
        }
    }).unwrap();

    if let Some(duration) = run_duration {
        println!("Running for {}s", duration.as_secs());
        libevent.run_timeout(duration);
    } else {
        // Do a few run_timeouts before running forever
        for _count in 1..=3 {
            let now = std::time::Instant::now();
            libevent.run_timeout(Duration::from_secs(5));

            let elapsed = now.elapsed();

            println!("Ran libevent loop for {:?}", elapsed);
        }

        println!("Running forever");
        libevent.run();
    }

    // TODO: expose base_free from libevent-rs
    let ret = unsafe { libevent.with_base(|base| ffi::helloc_destroy(base)) };
    assert_eq!(ret, 0);

    println!("Exiting");
}
