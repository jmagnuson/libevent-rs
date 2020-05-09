# hello

This example demonstrates how `libevent` and `libevent-sys` can be consumed by
both Rust as well as additional C code (using CMake in this case).

## Running

The example defaults to doing a self-compiled, statically-linked build via the
`bundled` feature flag. However, `pkg-config` can be used instead via
`--no-default-features`.

## Headers to C

The C portion (`helloc`) gets the libevent headers via cargo metadata provided
by `libevent-sys`, via the `DEP_EVENT_INCLUDE` environment variable. This is
why it is added as an additional, transitive dependency to the `libevent`
crate, since the latter does not propagate that information. More documentation
on how this works can be found in the [links manifest key] section of the build
scripts reference.

[links manifest key]: https://doc.rust-lang.org/cargo/reference/build-scripts.html#the-links-manifest-key
