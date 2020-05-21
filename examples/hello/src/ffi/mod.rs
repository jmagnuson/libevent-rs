use std::os::raw::c_int;

use libevent_sys;

#[link(name = "helloc")]
extern "C" {
    pub fn helloc_init(base: *mut libevent_sys::event_base) -> c_int;
    pub fn helloc_destroy(base: *mut libevent_sys::event_base) -> c_int;
}
