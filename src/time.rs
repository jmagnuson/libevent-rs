
use libevent_sys;
use std::time::Duration;

pub fn to_timeval(duration: Duration) -> libevent_sys::timeval {
     libevent_sys::timeval {
        tv_sec: duration.as_secs() as _,
        tv_usec: duration.subsec_micros() as _,
    }
}

