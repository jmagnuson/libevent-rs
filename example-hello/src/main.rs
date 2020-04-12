
use std::time::Duration;
use libevent::Libevent;

pub mod ffi;

fn main() {
    println!("Hello, world!");

    let libevent = Libevent::new()
        .unwrap_or_else(|e| panic!("{:?}", e));

    let _ = unsafe { libevent.with_base(|base| {
        ffi::helloc_init(base)
    })};

    let libevent_ref = &libevent;

    loop {
        let now = std::time::Instant::now();
        libevent_ref.loop_timeout(Duration::from_secs(5));

        let elapsed = now.elapsed();

        println!("Ran libevent loop for {:?}", elapsed);
    }
}
