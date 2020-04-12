
// use libc;
use libevent_sys;

use std::os::raw::c_int;
pub type evutil_socket_t = c_int;

//pub use evutil_socket_t;

#[link(name = "helloc")]
extern "C" {
    pub fn helloc_init(base: *mut libevent_sys::event_base) -> c_int;
    pub fn base_fd(base: *const libevent_sys::event_base) -> c_int;
    pub fn helloc_destroy(base: *mut libevent_sys::event_base) -> c_int;
    // pub fn register_tokio(base: *mut libevent_sys::event_base, fd: evutil_socket_t) -> libc::c_int;
}
