# libevent-rs

Rust bindings to the [libevent] async I/O framework.

## Usage

Add libevent to your `Cargo.toml`:

```toml
[dependencies]
libevent = "0.1"
```

### Sysem Requirements

* `libclang` is required by [bindgen] which is used to generate the Rust
  bindings. See [bindgen requirements] for more information.

* `cmake` if self-building via the `bundled` feature.

* `pkg-config` if not self-building via the `bundled` feature.

## Example

```rust,no_run
use libevent::Libevent;

let libevent = Libevent::new()?;

let mut count: usize = 0;

libevent.add_interval(
    Duration::from_secs(1),
    move |_flags| {
        count += 1;
        println!("count: {}", count);
    }
)?;

libevent.run();
```

For a more comprehensive example, see the [hello] project in the examples folder.

[libevent]: https://libevent.org/
[bindgen]: https://crates.io/crates/bindgen
[bindgen requirements]: https://rust-lang.github.io/rust-bindgen/requirements.html
[hello]: https://github.com/jmagnuson/libevent-rs/tree/hacking/examples/hello
