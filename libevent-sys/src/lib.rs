//! Raw Rust bindings to the `libevent` C library.
//!
//! Bindings are generated with [Rust-Bindgen](https://github.com/rust-lang/rust-bindgen)
//! which means there are a number of quirks.
//!
//! - Enums are a constants in the form of `enum_name_ENUM_FIELD`
//! - Functions are named the same as the C code and don't follow Rust naming schemes.
//! - Uses C strings. See `CStr` in the Rust standard library.

#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unreadable_literal)]
#![allow(deref_nullptr)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unaligned_references)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_access() {
        assert_eq!(EVENT_LOG_MSG, 1);
        assert_eq!(IPV6PORT_RESERVED, 1024);
    }
}
