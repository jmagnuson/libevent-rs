# libevent-rs

Rust bindings to the [libevent] async I/O framework.

## Example

```rust,no_run
use libevent::Libevent;

let libevent = Libevent::new()?;

let mut count: usize = 0;

libevent.add_interval(
    Duration::from_secs(1),
    move |_event, _flags| {
        count += 1;
        println!("count: {}", count);
    }
)?;

libevent.run();
```

### System Requirements

* `libclang` is required by [bindgen] which is used to generate the Rust
  bindings. See [bindgen requirements] for more information. Also ensure that
  `LIBCLANG_PATH` is set, as some systems do not do so by default.

* `cmake` if self-building via the `bundled` feature. The current bundled
  release is `release-2.1.11-stable`.

* `pkg-config` if not self-building via the `bundled` feature.


[libevent]: https://libevent.org/
[hacking]: https://github.com/jmagnuson/libevent-rs/tree/hacking
[bindgen]: https://crates.io/crates/bindgen
[bindgen requirements]: https://rust-lang.github.io/rust-bindgen/requirements.html
