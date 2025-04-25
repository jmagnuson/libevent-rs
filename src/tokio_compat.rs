use std::os::fd::RawFd;
use std::time::Duration;
use tokio::io::Interest;
use tokio::io::unix::AsyncFd;
use crate::{exit_code_to_reason, Base, ExitReason, LoopFlags};

pub struct AsyncBase {
    inner: Base,
    async_fd: AsyncFd<RawFd>,
}

use libevent_sys::internal::{self, event_base_loop_turn_pre, event_base_loop_pre, eventop, timeval, event_base_loop_turn_post, evthread_lock_fns_};

impl From<Base> for AsyncBase {
    fn from(base: Base) -> Self {
        let base_ptr = unsafe { (base.as_raw().as_ptr() as *mut internal::event_base) };
        let base_ref = unsafe { base_ptr.as_ref() }.unwrap();

        // TODO: just add a top-level get_fd function to avoid casting to internal::ev_base

        let evsel = unsafe { base_ref.evsel.as_ref().unwrap() };
        let fd = unsafe { evsel.event_fd.unwrap()(base_ptr) } as RawFd;
        // TODO: don't unwrap
        let async_fd = AsyncFd::with_interest(fd, Interest::READABLE).unwrap();
        AsyncBase {
            inner: base,
            async_fd,
        }
    }
}

impl AsyncBase {
    pub async fn _loop(&mut self) -> ExitReason {
        // cast needed to access evsel
        let base_ptr = unsafe { (self.inner.as_raw().as_ptr() as *mut internal::event_base) };
        let base_ref = unsafe { base_ptr.as_mut() }.unwrap();
        let evsel = unsafe { base_ref.evsel.as_ref().unwrap() };
        let flags = 0;
        let mut done = 0;
        let done_ptr = &mut done as *mut _;
        let mut retval: i32 = 0; //ExitReason::GotExit; // not sure if this is right
        let mut tv: timeval = timeval { tv_sec: 0, tv_usec: 0 };
        let mut tv_p: *mut timeval;
        let mut res = 0;

        let mut ret = unsafe { event_base_loop_pre(base_ptr, flags, done_ptr) };
        if ret != 0 {
            return exit_code_to_reason(unsafe { self.inner.as_raw() }, ret, LoopFlags::from_bits(flags as u32).unwrap());
        }

        let mut timeout_count = 0;
        while done == 0 {
            tv_p = &mut tv;
            ret = unsafe { event_base_loop_turn_pre(
                base_ptr, flags, &mut retval, &mut tv as *mut _, &mut tv_p,
            )};
            if ret == 2 {
                // break
                break;
            } else if ret == 1 {
                // goto done
                break;
            }
            let timeout = unsafe { evsel.dispatch_pre.unwrap()(base_ptr, tv_p) };
            let timeout_fut = if timeout < 0 {
                // FIXME: but timeout < 0 should be an error?
                // println!("************* Timeout is < 0!, timeout={timeout}, timeout_count={timeout_count}, tv:{tv:?}, tv_p:{tv_p:?} **************");
                tokio_util::either::Either::Left(std::future::pending())
            } else {
                tokio_util::either::Either::Right(tokio::time::sleep(Duration::from_millis(timeout as u64)))
            };
            let ep_readable = self.async_fd.readable();
            let wait_res = tokio::select! {
                _ = timeout_fut => {
                    timeout_count += 1;
                    let wait_res = unsafe { evsel.dispatch_wait.unwrap()(base_ptr, 0) };
                    wait_res
                }
                rguard = ep_readable => {
                    let wait_res = unsafe { evsel.dispatch_wait.unwrap()(base_ptr, 0) };
                    //if wait_res == 0 {
                        rguard.unwrap().clear_ready();
                    //}
                    wait_res
                }
            };
            res = unsafe { evsel.dispatch_post.unwrap()(base_ptr, wait_res) };

            ret = unsafe { event_base_loop_turn_post(base_ptr, flags, res, done_ptr, &mut retval as *mut _) };
            if ret == 1 {
                // goto done
                break;
            }
        }

        clear_time_cache(base_ref);
        base_ref.running_loop = 0;
        // no C macros yet:
        // https://github.com/rust-lang/rust-bindgen/issues/753
        if !base_ref.th_base_lock.is_null() {
            unsafe {
                evthread_lock_fns_.unlock.unwrap()(0, base_ref.th_base_lock);
            }
        }

        exit_code_to_reason(unsafe { self.inner.as_raw() }, retval, LoopFlags::from_bits(flags as u32).unwrap())
    }
}

fn clear_time_cache(base: &mut internal::event_base) {
    base.tv_cache.tv_sec = 0;
}
