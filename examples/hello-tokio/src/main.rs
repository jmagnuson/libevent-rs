use libevent::{Base, Interval};
use std::time::Duration;
use tokio::task::LocalSet;
use libevent::{EventCallbackCtx, EventCallbackFlags, EvutilSocket};

extern "C" fn hello_callback(
    _fd: EvutilSocket,
    _event: EventCallbackFlags,
    _ctx: EventCallbackCtx,
) {
    println!("callback: rust fn (interval: 2s)");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let run_duration = std::env::args().nth(1).map(|val_s| {
        Duration::from_secs(
            val_s
                .parse()
                .expect("Bad run duration argument (should be in seconds)"),
        )
    });

    let mut base = Base::new().unwrap_or_else(|e| panic!("{:?}", e));

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

    let local_set = LocalSet::new();
    local_set
        .run_until(async move {
            let mut abase = libevent::tokio_compat::AsyncBase::from(base);
            let timeout = tokio::time::sleep(Duration::from_secs(10));

                    tokio::select!{
                _ = abase._loop() => {}
                _ = timeout => {}
            }
        }).await;

    println!("Exiting");
}
