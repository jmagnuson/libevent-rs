# libevent-sys
[![](https://meritbadge.herokuapp.com/libevent-sys)](https://crates.io/crates/libevent-sys)
[![Released API docs](https://docs.rs/libevent-sys/badge.svg)](https://docs.rs/libevent-sys)

Rust FFI bindings to [libevent] library made using [Rust-Bindgen].

## System Requirements

* `libclang` is required by [bindgen] which is used to generate the Rust
  bindings. See [bindgen requirements] for more information. Also ensure that
  `LIBCLANG_PATH` is set, as some systems do not do so by default.

* `cmake` if self-building via the `bundled` feature. The current bundled
  release is `release-2.1.11-stable`.

* `pkg-config` if not self-building via the `bundled` feature.


## Building
Depends on `libevent-dev` or equivalent to be installed on the system.
It can be found in most distro's package managers or from the `libevent`
website linked above.

Once that is installed just use `cargo build`.

[libevent]: https://libevent.org/
[Rust-Bindgen]: https://github.com/rust-lang/rust-bindgen
[bindgen]: https://crates.io/crates/bindgen
[bindgen requirements]: https://rust-lang.github.io/rust-bindgen/requirements.html

