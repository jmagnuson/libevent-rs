use libevent::{Base, Interval};
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

#[cfg(feature = "tokio_backend")]
fn inject_tokio(base: &Base) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build a tokio runtime");

    runtime.spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("'Hello, world' from a tokio task!");
        }
    });

    base.inject_tokio(runtime);
}

fn main() {
    #[cfg(feature = "tracing_subscriber")]
    tracing_subscriber::fmt::init();

    let run_duration = std::env::args().nth(1).map(|val_s| {
        Duration::from_secs(
            val_s
                .parse()
                .expect("Bad run duration argument (should be in seconds)"),
        )
    });

    let mut base = Base::new().unwrap_or_else(|e| panic!("{:?}", e));

    #[cfg(feature = "tokio_backend")]
    inject_tokio(&base);

    let ret = unsafe { ffi::helloc_init(base.as_raw().as_ptr()) };
    assert_eq!(ret, 0);

    let ev = base
        .event_new(None, libevent::EventFlags::PERSIST, hello_callback, None)
        .expect("Faled to allocate event");

    base.event_add(ev, Some(Duration::from_secs(2)));

    let mut b: usize = 0;
    let ev = Interval::new(Duration::from_secs(2));
    let mut ev_handle = Some(
        base.spawn_local(ev, move |_ev| {
            b += 1;
            println!(
                "callback (b): rust closure (interval: 2s, count: {}, flags: {:?})",
                b, "TIMEOUT"
            );
        })
        .unwrap_or_else(|e| panic!("{:?}", e)),
    );

    {
        let mut a: usize = 0;

        let ev = Interval::new(Duration::from_secs(3));

        base.spawn(ev, move |_ev| {
            a += 1;
            println!(
                "callback: rust closure (interval: 3s, count: {}, flags: {:?})",
                a, "TIMEOUT"
            );

            if a > 3 {
                println!("callback: rust closure (STOPPING)");
                _ev.stop().unwrap_or_else(|e| panic!("{:?}", e));

                // drop the event handle for b
                let _ = ev_handle.take();
            }
        })
        .unwrap_or_else(|e| panic!("{:?}", e));
    }

    if let Some(duration) = run_duration {
        println!("Running for {}s", duration.as_secs());
        base.run_timeout(duration);
    } else {
        // Do a few run_timeouts before running forever
        for _count in 1..=3 {
            let now = std::time::Instant::now();
            base.run_timeout(Duration::from_secs(5));

            let elapsed = now.elapsed();

            println!("Ran libevent loop for {:?}", elapsed);
        }

        println!("Running forever");
        base.run();
    }

    // TODO: expose base_free from libevent-rs
    let ret = unsafe { ffi::helloc_destroy(base.as_raw().as_ptr()) };
    assert_eq!(ret, 0);

    println!("Exiting");
}
