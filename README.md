# libevent-rs

Rust bindings to the [libevent] async I/O framework.

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

[libevent]: https://libevent.org/
