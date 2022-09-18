# libevent-rs
[![](https://meritbadge.herokuapp.com/libevent)](https://crates.io/crates/libevent)
[![Released API docs](https://docs.rs/libevent/badge.svg)](https://docs.rs/libevent)

Rust bindings to the [libevent] async I/O framework.

## Example

```rust,no_run
use libevent::{Base, Interval};

let mut base = Base::new();

let mut count: usize = 0;

let timer = Interval::new(Duration::from_secs(1));

base.spawn(timer, move |_event| {
    count += 1;
    println!("count: {}", count);
})?;

base.run();
```

## System Requirements

* `libclang` is required by [bindgen] which is used to generate the Rust
  bindings. See [bindgen requirements] for more information. Also ensure that
  `LIBCLANG_PATH` is set, as some systems do not do so by default. `libclang`
  is only required if `buildtime_bindgen` is enabled.

* `cmake` if self-building via the `bundled` feature. The current bundled
  release is `release-2.1.11-stable`.

* `pkg-config` if not self-building via the `bundled` feature.

* `buildtime_bindgen` is an optional feature, enabled by default, which
  indicates that the Rust libevent bindings should be generated at build time.

* `LIBEVENT_SYS_BINDGEN_FILE` is an environment variable indicating the path of
  the file containing the pregenerated Rust bindings which must be populated
  when `buildtime_bindgen` is not enabled, and it is only applicable in this
  case.

## Minimum Supported Rust Version (MSRV)

This crate is guaranteed to compile on stable Rust 1.57.0 and up. It might compile
with older versions but that may change in any new patch release.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[libevent]: https://libevent.org/
[hacking]: https://github.com/jmagnuson/libevent-rs/tree/hacking
[bindgen]: https://crates.io/crates/bindgen
[bindgen requirements]: https://rust-lang.github.io/rust-bindgen/requirements.html
