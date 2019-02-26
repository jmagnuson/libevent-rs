# libevent-sys
[![](https://meritbadge.herokuapp.com/libevent-sys)](https://crates.io/crates/libevent-sys)
[![Released API docs](https://docs.rs/libevent-sys/badge.svg)](https://docs.rs/libevent-sys)

Rust FFI bindings to [libevent](https://libevent.org/) library made using [Rust-Bindgen](https://github.com/rust-lang/rust-bindgen).

Created and maintained by the [Tornado Project](https://gitlab.com/tornado-torrent/)

**Note this package only supports Linux at this time.**

Support for MacOS and Windows is planned, but pull requests helping
are greatly appreciated.
Especially for MacOS because I do not have access to one.

## Building
Depends on `libevent-dev` or equivalent to be installed on the system.
It can be found in most distro's package managers or from the `libevent`
website linked aboce.

Once that is installed just use `cargo build`.